use crate::lightning;
use crate::{HelipadConfig, REMOTE_GUID_CACHE_SIZE};
use lru::LruCache;
use reqwest;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::error::Error;
use std::num::NonZeroUsize;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct RawBoost {
    #[serde(default = "d_action")]
    action: Option<String>,
    #[serde(default = "d_blank")]
    app_name: Option<String>,
    #[serde(default = "d_blank")]
    app_version: Option<String>,
    #[serde(default = "d_blank")]
    boost_link: Option<String>,
    #[serde(default = "d_blank")]
    message: Option<String>,
    #[serde(default = "d_blank")]
    name: Option<String>,
    #[serde(default = "d_blank")]
    pubkey: Option<String>,
    #[serde(default = "d_blank")]
    sender_key: Option<String>,
    #[serde(default = "d_blank")]
    sender_name: Option<String>,
    #[serde(default = "d_blank")]
    sender_id: Option<String>,
    #[serde(default = "d_blank")]
    sig_fields: Option<String>,
    #[serde(default = "d_blank")]
    signature: Option<String>,
    #[serde(default = "d_blank")]
    speed: Option<String>,
    #[serde(default = "d_blank")]
    uuid: Option<String>,
    #[serde(default = "d_blank")]
    podcast: Option<String>,
    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    feedID: Option<u64>,
    #[serde(default = "d_blank")]
    guid: Option<String>,
    #[serde(default = "d_blank")]
    url: Option<String>,
    #[serde(default = "d_blank")]
    episode: Option<String>,
    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    itemID: Option<u64>,
    #[serde(default = "d_blank")]
    episode_guid: Option<String>,
    #[serde(default = "d_blank")]
    time: Option<String>,
    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    ts: Option<u64>,
    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    value_msat: Option<u64>,
    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    value_msat_total: Option<u64>,
    #[serde(default = "d_blank")]
    remote_feed_guid: Option<String>,
    #[serde(default = "d_blank")]
    remote_item_guid: Option<String>,
}

fn d_action() -> Option<String> {
    Some("stream".to_string())
}

fn d_blank() -> Option<String> {
    None
}

fn d_zero() -> Option<u64> {
    None
}

fn de_optional_string_or_number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u64>, D::Error> {
    Ok(
        match Value::deserialize(deserializer)? {
            Value::String(s) => {
                if s.is_empty() {
                    return Ok(None);
                }
                if let Ok(number) = s.parse() {
                    Some(number)
                } else {
                    return Ok(None);
                }
            }
            Value::Number(num) => {
                if num.is_u64() {
                    if let Some(number) = num.as_u64() {
                        Some(number)
                    } else {
                        return Ok(None);
                    }
                } else {
                    return Ok(None);
                }

            }
            _ => Some(0)
        }
    )
}


#[derive(Clone, Debug)]
pub struct PodcastEpisodeGuid {
    pub podcast_guid: String,
    pub episode_guid: String,
    pub podcast: String,
    pub episode: String,
}

// Fetches remote podcast/episode names by guids using the Podcastindex API and caches results into an LRU cache
pub async fn fetch_podcast_episode_by_guid(cache: &mut LruCache<String, Option<PodcastEpisodeGuid>>, podcast_guid: String, episode_guid: String) -> Option<PodcastEpisodeGuid> {
    let key = format!("{}_{}", podcast_guid, episode_guid);

    if let Some(cached_guid) = cache.get(&key) {
        println!("Remote podcast/episode from cache: {:#?}", cached_guid);
        return cached_guid.clone(); // already exists in cache
    }

    match fetch_api_podcast_episode_by_guid(&podcast_guid, &episode_guid).await {
        Ok(Some(guid)) => {
            println!("Remote podcast/episode from API: {:#?}", guid);
            cache.put(key, Some(guid.clone())); // cache to avoid spamming api
            Some(guid)
        },
        Ok(None) => {
            println!("Remote podcast/episode not found {} {}", podcast_guid, episode_guid);
            cache.put(key, None); // cache to avoid spamming api
            None
        }
        Err(e) => {
            eprintln!("Error retrieving remote podcast/episode from API: {:#?}", e);
            None
        }
    }
}

// Fetches remote podcast/episode names by guids using the Podcastindex API
pub async fn fetch_api_podcast_episode_by_guid(podcast_guid: &String, episode_guid: &String) -> Result<Option<PodcastEpisodeGuid>, Box<dyn Error>> {
    let query = vec![
        ("podcastguid", podcast_guid),
        ("episodeguid", episode_guid)
    ];

    let app_version = env!("CARGO_PKG_VERSION");

    // call API, get text response, and parse into json
    let response = reqwest::Client::new()
        .get("https://api.podcastindex.org/api/1.0/value/byepisodeguid")
        .header(USER_AGENT, format!("Helipad/{}", app_version))
        .query(&query)
        .send()
        .await?;

    let result = response.text().await?;
    let json: Value = serde_json::from_str(&result)?;

    let status = json["status"].as_str().unwrap_or_default();

    if status != "true" {
        return Ok(None); // not found?
    }

    let query = match json["query"].as_object() {
        Some(val) => val,
        None => { return Ok(None); }
    };

    let value = match json["value"].as_object() {
        Some(val) => val,
        None => { return Ok(None); }
    };

    let found_podcast_guid = query["podcastguid"].as_str().unwrap_or_default();
    let found_episode_guid = query["episodeguid"].as_str().unwrap_or_default();

    let found_podcast = value["feedTitle"].as_str().unwrap_or_default();
    let found_episode = value["title"].as_str().unwrap_or_default();

    return Ok(Some(PodcastEpisodeGuid {
        podcast_guid: found_podcast_guid.to_string(),
        episode_guid: found_episode_guid.to_string(),
        podcast: found_podcast.to_string(),
        episode: found_episode.to_string(),
    }))
}

pub async fn parse_podcast_tlv(boost: &mut dbif::BoostRecord, val: &Vec<u8>, remote_cache: &mut LruCache<String, Option<PodcastEpisodeGuid>>) {
    let tlv = std::str::from_utf8(&val).unwrap();
    println!("TLV: {:#?}", tlv);

    boost.tlv = tlv.to_string();

    let json_result = serde_json::from_str::<RawBoost>(tlv);
    match json_result {
        Ok(rawboost) => {
            //If there was a sat value in the tlv, override the invoice
            if rawboost.value_msat.is_some() {
                boost.value_msat = rawboost.value_msat.unwrap() as i64;
            }

            //Determine an action type for later filtering ability
            if rawboost.action.is_some() {
                boost.action = match rawboost.action.unwrap().as_str() {
                    "stream" => 1, //This indicates a per-minute podcast payment
                    "boost" => 2,  //This is a manual boost or boost-a-gram
                    _ => 3,
                }
            }

            //Was a sender name given in the tlv?
            if rawboost.sender_name.is_some() && !rawboost.sender_name.clone().unwrap().is_empty() {
                boost.sender = rawboost.sender_name.unwrap();
            }

            //Was there a message in this tlv?
            if rawboost.message.is_some() {
                boost.message = rawboost.message.unwrap();
            }

            //Was an app name given?
            if rawboost.app_name.is_some() {
                boost.app = rawboost.app_name.unwrap();
            }

            //Was a podcast name given?
            if rawboost.podcast.is_some() {
                boost.podcast = rawboost.podcast.unwrap();
            }

            //Episode name?
            if rawboost.episode.is_some() {
                boost.episode = rawboost.episode.unwrap();
            }

            //Look for an original sat value in the tlv
            if rawboost.value_msat_total.is_some() {
                boost.value_msat_total = rawboost.value_msat_total.unwrap() as i64;
            }

            //Fetch podcast/episode name if remote feed/item guid present
            if rawboost.remote_feed_guid.is_some() && rawboost.remote_item_guid.is_some() {
                let remote_feed_guid = rawboost.remote_feed_guid.unwrap();
                let remote_item_guid = rawboost.remote_item_guid.unwrap();

                let episode_guid = fetch_podcast_episode_by_guid(remote_cache, remote_feed_guid, remote_item_guid).await;

                if let Some(guid) = episode_guid {
                    boost.remote_podcast = Some(guid.podcast);
                    boost.remote_episode = Some(guid.episode);
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}

//The LND poller runs in a thread and pulls new invoices
pub async fn lnd_poller(helipad_config: HelipadConfig) {
    let db_filepath = helipad_config.database_file_path.clone();

    //Make the connection to LND
    println!("\nConnecting to LND node address...");
    let mut lightning;
    match lightning::connect_to_lnd(helipad_config.node_address, helipad_config.cert_path, helipad_config.macaroon_path).await {
        Some(lndconn) => {
            println!(" - Success.");
            lightning = lndconn;
        }
        None => {
            std::process::exit(1);
        }
    }

    //Get lnd node info
    match lnd::Lnd::get_info(&mut lightning).await {
        Ok(node_info) => {
            println!("LND node info: {:#?}", node_info);
        }
        Err(e) => {
            eprintln!("Error getting LND node info: {:#?}", e);
        }
    }

    //Instantiate a cache to use when resolving remote podcasts/episode guids
    let mut remote_cache = LruCache::new(NonZeroUsize::new(REMOTE_GUID_CACHE_SIZE).unwrap());

    //The main loop
    let mut current_index = dbif::get_last_boost_index_from_db(&db_filepath).unwrap();
    let mut current_payment = dbif::get_last_payment_index_from_db(&db_filepath).unwrap();
    loop {

        //Get lnd node channel balance
        match lnd::Lnd::channel_balance(&mut lightning).await {
            Ok(balance) => {
                let mut current_balance: i64 = 0;
                if let Some(bal) = balance.local_balance {
                    println!("LND node local balance: {:#?}", bal.sat);
                    current_balance = bal.sat as i64;
                }

                if dbif::add_wallet_balance_to_db(&db_filepath, current_balance).is_err() {
                    println!("Error adding wallet balance to the database.");
                }
            }
            Err(e) => {
                eprintln!("Error getting LND wallet balance: {:#?}", e);
            }
        }

        //Get a list of invoices
        match lnd::Lnd::list_invoices(&mut lightning, false, current_index.clone(), 500, false).await {
            Ok(response) => {
                for invoice in response.invoices {

                    //Initialize a boost record
                    let mut boost = dbif::BoostRecord {
                        index: invoice.add_index,
                        time: invoice.settle_date,
                        value_msat: invoice.amt_paid_sat * 1000,
                        value_msat_total: invoice.amt_paid_sat * 1000,
                        action: 0,
                        sender: "".to_string(),
                        app: "".to_string(),
                        message: "".to_string(),
                        podcast: "".to_string(),
                        episode: "".to_string(),
                        tlv: "".to_string(),
                        remote_podcast: None,
                        remote_episode: None,
                        payment_info: None,
                    };

                    //Search for podcast boost tlvs
                    for htlc in invoice.htlcs {
                        for (idx, val) in htlc.custom_records {
                            //Satoshis.stream record type
                            if idx == lightning::TLV_PODCASTING20 {
                                parse_podcast_tlv(&mut boost, &val, &mut remote_cache).await;
                            }
                        }
                    }

                    //Give some output
                    println!("Boost: {:#?}", boost);

                    //Store in the database
                    println!("{:#?}", boost);
                    match dbif::add_invoice_to_db(&db_filepath, boost) {
                        Ok(_) => println!("New invoice added."),
                        Err(e) => eprintln!("Error adding invoice: {:#?}", e)
                    }
                }
            }
            Err(e) => {
                eprintln!("lnd::Lnd::list_invoices failed: {}", e);
            }
        }

        //Make sure we are tracking our position properly
        current_index = dbif::get_last_boost_index_from_db(&db_filepath).unwrap();
        println!("Current index: {}", current_index);

        match lnd::Lnd::list_payments(&mut lightning, false, current_payment, 500, false).await {
            Ok(response) => {
                for payment in response.payments {

                    for htlc in payment.htlcs {

                        if let Some(route) = htlc.route {
                            let hopidx = route.hops.len() - 1;
                            let hop = route.hops[hopidx].clone();

                            if !hop.custom_records.contains_key(&lightning::TLV_PODCASTING20) {
                                continue; // not a boost payment
                            }

                            //Initialize a boost record
                            let mut boost = dbif::BoostRecord {
                                index: payment.payment_index,
                                time: payment.creation_time_ns / 1000000000,
                                value_msat: payment.value_msat,
                                value_msat_total: payment.value_msat,
                                action: 0,
                                sender: "".to_string(),
                                app: "".to_string(),
                                message: "".to_string(),
                                podcast: "".to_string(),
                                episode: "".to_string(),
                                tlv: "".to_string(),
                                remote_podcast: None,
                                remote_episode: None,
                                payment_info: Some(dbif::PaymentRecord {
                                    pubkey: hop.pub_key.clone(),
                                    custom_key: 0,
                                    custom_value: "".into(),
                                    fee_msat: payment.fee_msat,
                                }),
                            };

                            for (idx, val) in hop.custom_records {
                                if idx == lightning::TLV_PODCASTING20 {
                                    parse_podcast_tlv(&mut boost, &val, &mut remote_cache).await;
                                }
                                else if idx == lightning::TLV_WALLET_KEY || idx == lightning::TLV_WALLET_ID || idx == lightning::TLV_HIVE_ACCOUNT {
                                    let custom_value = std::str::from_utf8(&val).unwrap().to_string();

                                    boost.payment_info = Some(dbif::PaymentRecord {
                                        pubkey: hop.pub_key.clone(),
                                        custom_key: idx,
                                        custom_value: custom_value,
                                        fee_msat: payment.fee_msat,
                                    });
                                }
                            }

                            //Give some output
                            println!("Sent Boost: {:#?}", boost);

                            match dbif::add_payment_to_db(&db_filepath, boost) {
                                Ok(_) => println!("New payment added."),
                                Err(e) => eprintln!("Error adding payment: {:#?}", e)
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("lnd::Lnd::list_payments failed: {}", e);
            }
        };

        //Make sure we are tracking our position properly
        current_payment = dbif::get_last_payment_index_from_db(&db_filepath).unwrap();
        println!("Current payment: {}", current_payment);

        tokio::time::sleep(tokio::time::Duration::from_millis(9000)).await;
    }
}