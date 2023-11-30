use crate::{Context, Request, Body, Response, connect_to_lnd, resolve_lightning_address, send_boost, LnAddressResponse};
use hyper::StatusCode;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::str;
use voca_rs::*;
use handlebars::Handlebars;
use serde_json::json;
use chrono::{NaiveDateTime};
use dbif::BoostRecord;
use data_encoding::HEXLOWER;

//Constants --------------------------------------------------------------------------------------------------
const WEBROOT_PATH_HTML: &str = "webroot/html";
const WEBROOT_PATH_IMAGE: &str = "webroot/image";
const WEBROOT_PATH_STYLE: &str = "webroot/style";
const WEBROOT_PATH_SCRIPT: &str = "webroot/script";


//Structs and Enums ------------------------------------------------------------------------------------------
#[derive(Debug)]
struct HydraError(String);

impl fmt::Display for HydraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fatal error: {}", self.0)
    }
}

impl Error for HydraError {}

//Helper functions
fn get_query_params(req: Request<Body>) -> HashMap<String, String> {
    return req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);
}

async fn get_post_params(req: Request<Body>) -> HashMap<String, String> {
    let full_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
    let body_str = str::from_utf8(&full_body).unwrap();
    let body_params = url::form_urlencoded::parse(body_str.as_bytes());

    return body_params
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
}

fn client_error_response(message: String) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(message.into())
        .unwrap();
}

fn server_error_response(message: String) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .body(message.into())
        .unwrap();
}

fn json_response<T: serde::Serialize>(value: T) -> Response {
    let json_doc = serde_json::to_string_pretty(&value).unwrap();
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "application/json; charset=UTF-8")
        .body(format!("{}", json_doc).into())
        .unwrap();
}

fn options_response(options: String) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header("Access-Control-Allow-Methods", options)
        .body(format!("").into())
        .unwrap();
}

//Route handlers ---------------------------------------------------------------------------------------------

//Homepage html
pub async fn home(ctx: Context) -> Response {

    //Get query parameters
    let _params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    let reg = Handlebars::new();
    let doc = fs::read_to_string("webroot/html/home.html").expect("Something went wrong reading the file.");
    let doc_rendered = reg.render_template(&doc, &json!({"version": ctx.state.version})).expect("Something went wrong rendering the file");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc_rendered).into())
        .unwrap();
}

//Streams html
pub async fn streams(ctx: Context) -> Response {

    //Get query parameters
    let _params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    let reg = Handlebars::new();
    let doc = fs::read_to_string("webroot/html/streams.html").expect("Something went wrong reading the file.");
    let doc_rendered = reg.render_template(&doc, &json!({"version": ctx.state.version})).expect("Something went wrong rendering the file");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc_rendered).into())
        .unwrap();
}

//Sent html
pub async fn sent(ctx: Context) -> Response {

    //Get query parameters
    let _params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    let reg = Handlebars::new();
    let doc = fs::read_to_string("webroot/html/sent.html").expect("Something went wrong reading the file.");
    let doc_rendered = reg.render_template(&doc, &json!({"version": ctx.state.version})).expect("Something went wrong rendering the file");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc_rendered).into())
        .unwrap();
}

//Pew-pew audio
pub async fn pewmp3(_ctx: Context) -> Response {
    let file = fs::read("webroot/extra/pew.mp3").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-type", "audio/mpeg")
        .body(hyper::Body::from(file))
        .unwrap();
}

//Favicon icon
pub async fn favicon(_ctx: Context) -> Response {
    let file = fs::read("webroot/extra/favicon.ico").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-type", "image/x-icon")
        .body(hyper::Body::from(file))
        .unwrap();
}

//Apps definitions file
pub async fn apps_json(_ctx: Context) -> Response {
    let file = fs::read("webroot/extra/apps.json").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-type", "application/json; charset=utf-8")
        .body(hyper::Body::from(file))
        .unwrap();
}

//Numerology definitions file
pub async fn numerology_json(_ctx: Context) -> Response {
    let file = fs::read("webroot/extra/numerology.json").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-type", "application/json; charset=utf-8")
        .body(hyper::Body::from(file))
        .unwrap();
}

//Serve a web asset by name from webroot subfolder according to it's requested type
pub async fn asset(ctx: Context) -> Response {
    //Get query parameters
    let _params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    println!("** Context: {:#?}", ctx);
    println!("** Params: {:#?}", _params);

    //Set up the response framework
    let file_path;
    let content_type;
    let file_extension;
    match ctx.path.as_str() {
        "/html" => {
            file_path = WEBROOT_PATH_HTML;
            content_type = "text/html";
            file_extension = "html";
        }
        "/image" => {
            file_path = WEBROOT_PATH_IMAGE;
            content_type = "image/png";
            file_extension = "png";
        }
        "/style" => {
            file_path = WEBROOT_PATH_STYLE;
            content_type = "text/css";
            file_extension = "css";
        }
        "/script" => {
            file_path = WEBROOT_PATH_SCRIPT;
            content_type = "text/javascript";
            file_extension = "js";
        }
        _ => {
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** Invalid asset type requested (ex. /images?name=filename.").into())
                .unwrap();
        }
    };

    //Attempt to serve the file
    if let Some(filename) = _params.get("name") {
        let file_to_serve = format!("{}/{}.{}", file_path, filename, file_extension);
        println!("** Serving file: [{}]", file_to_serve);
        let file = fs::read(file_to_serve.as_str()).expect("Something went wrong reading the file.");
        return hyper::Response::builder()
            .status(StatusCode::OK)
            .header("Content-type", content_type)
            .body(hyper::Body::from(file))
            .unwrap();
    } else {
        return hyper::Response::builder()
            .status(StatusCode::from_u16(500).unwrap())
            .body(format!("** No file specified.").into())
            .unwrap();
    }
}

//API - give back the node balance
pub async fn api_v1_balance_options(_ctx: Context) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::from_u16(204).unwrap())
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .body(format!("").into())
        .unwrap();
}

pub async fn api_v1_balance(_ctx: Context) -> Response {
    //Get query parameters
    let _params: HashMap<String, String> = _ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    //Get the boosts from db for returning
    match dbif::get_wallet_balance_from_db(&_ctx.helipad_config.database_file_path) {
        Ok(balance) => {
            let json_doc = serde_json::to_string_pretty(&balance).unwrap();

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .body(format!("{}", json_doc).into())
                .unwrap();
        }
        Err(e) => {
            eprintln!("** Error getting balance: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("** Error getting balance.").into())
                .unwrap();
        }
    }
}

//API - serve boosts as JSON either in ascending or descending order
pub async fn api_v1_boosts_options(_ctx: Context) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::from_u16(204).unwrap())
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .body(format!("").into())
        .unwrap();
}

pub async fn api_v1_boosts(_ctx: Context) -> Response {
    //Get query parameters
    let params: HashMap<String, String> = _ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    //Parameter - index (unsigned int)
    let index: u64;
    match params.get("index") {
        Some(supplied_index) => {
            index = match supplied_index.parse::<u64>() {
                Ok(index) => {
                    println!("** Supplied index from call: [{}]", index);
                    index
                }
                Err(_) => {
                    eprintln!("** Error getting boosts: 'index' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting boosts: 'index' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Parameter - boostcount (unsigned int)
    let boostcount: u64;
    match params.get("count") {
        Some(bcount) => {
            boostcount = match bcount.parse::<u64>() {
                Ok(boostcount) => {
                    println!("** Supplied boostcount from call: [{}]", boostcount);
                    boostcount
                }
                Err(_) => {
                    eprintln!("** Error getting boosts: 'count' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting boosts: 'count' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Was the "old" flag used?
    let mut old = false;
    match params.get("old") {
        Some(_) => old = true,
        None => {}
    };

    //Get the boosts from db for returning
    match dbif::get_boosts_from_db(&_ctx.helipad_config.database_file_path, index, boostcount, old, true) {
        Ok(boosts) => {
            let json_doc = serde_json::to_string_pretty(&boosts).unwrap();

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .body(format!("{}", json_doc).into())
                .unwrap();
        }
        Err(e) => {
            eprintln!("** Error getting boosts: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("** Error getting boosts.").into())
                .unwrap();
        }
    }
}

//API - serve streams as JSON either in ascending or descending order
pub async fn api_v1_streams_options(_ctx: Context) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::from_u16(204).unwrap())
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .body(format!("").into())
        .unwrap();
}

pub async fn api_v1_streams(_ctx: Context) -> Response {
    //Get query parameters
    let params: HashMap<String, String> = _ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    //Parameter - index (unsigned int)
    let index: u64;
    match params.get("index") {
        Some(supplied_index) => {
            index = match supplied_index.parse::<u64>() {
                Ok(index) => {
                    println!("** Supplied index from call: [{}]", index);
                    index
                }
                Err(_) => {
                    eprintln!("** Error getting streams: 'index' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting streams: 'index' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Parameter - boostcount (unsigned int)
    let boostcount: u64;
    match params.get("count") {
        Some(bcount) => {
            boostcount = match bcount.parse::<u64>() {
                Ok(boostcount) => {
                    println!("** Supplied stream count from call: [{}]", boostcount);
                    boostcount
                }
                Err(_) => {
                    eprintln!("** Error getting streams: 'count' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting streams: 'count' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Was the "old" flag used?
    let mut old = false;
    match params.get("old") {
        Some(_) => old = true,
        None => {}
    };

    //Get the boosts from db for returning
    match dbif::get_streams_from_db(&_ctx.helipad_config.database_file_path, index, boostcount, old) {
        Ok(streams) => {
            let json_doc_raw = serde_json::to_string_pretty(&streams).unwrap();
            let json_doc: String = strip::strip_tags(&json_doc_raw);

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .body(format!("{}", json_doc).into())
                .unwrap();
        }
        Err(e) => {
            eprintln!("** Error getting streams: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("** Error getting streams.").into())
                .unwrap();
        }
    }
}

//API - get the current invoice index number
pub async fn api_v1_index_options(_ctx: Context) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::from_u16(204).unwrap())
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .body(format!("").into())
        .unwrap();
}

pub async fn api_v1_index(_ctx: Context) -> Response {

    //Get the last known invoice index from the database
    match dbif::get_last_boost_index_from_db(&_ctx.helipad_config.database_file_path) {
        Ok(index) => {
            println!("** get_last_boost_index_from_db() -> [{}]", index);
            let json_doc_raw = serde_json::to_string_pretty(&index).unwrap();
            let json_doc: String = strip::strip_tags(&json_doc_raw);

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .body(format!("{}", json_doc).into())
                .unwrap();
        }
        Err(e) => {
            eprintln!("** Error getting current db index: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("** Error getting current db index.").into())
                .unwrap();
        }
    };
}

//API - get the current payment index number
pub async fn api_v1_sent_index_options(_ctx: Context) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::from_u16(204).unwrap())
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .body(format!("").into())
        .unwrap();
}

pub async fn api_v1_sent_index(_ctx: Context) -> Response {
    //Get the last known payment index from the database
    match dbif::get_last_payment_index_from_db(&_ctx.database_file_path) {
        Ok(index) => {
            println!("** get_last_payment_index_from_db() -> [{}]", index);
            let json_doc_raw = serde_json::to_string_pretty(&index).unwrap();
            let json_doc: String = strip::strip_tags(&json_doc_raw);

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .body(format!("{}", json_doc).into())
                .unwrap();
        }
        Err(e) => {
            eprintln!("** Error getting current db index: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("** Error getting current db index.").into())
                .unwrap();
        }
    };
}


//API - serve sent as JSON either in ascending or descending order
pub async fn api_v1_sent_options(_ctx: Context) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::from_u16(204).unwrap())
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .body(format!("").into())
        .unwrap();
}

pub async fn api_v1_sent(_ctx: Context) -> Response {
    //Get query parameters
    let params: HashMap<String, String> = _ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    //Parameter - index (unsigned int)
    let index: u64;
    match params.get("index") {
        Some(supplied_index) => {
            index = match supplied_index.parse::<u64>() {
                Ok(index) => {
                    println!("** Supplied index from call: [{}]", index);
                    index
                }
                Err(_) => {
                    eprintln!("** Error getting sent boosts: 'index' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting sent boosts: 'index' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Parameter - boostcount (unsigned int)
    let boostcount: u64;
    match params.get("count") {
        Some(bcount) => {
            boostcount = match bcount.parse::<u64>() {
                Ok(boostcount) => {
                    println!("** Supplied sent boost count from call: [{}]", boostcount);
                    boostcount
                }
                Err(_) => {
                    eprintln!("** Error getting sent boosts: 'count' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting sent boosts: 'count' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Was the "old" flag used?
    let mut old = false;
    match params.get("old") {
        Some(_) => old = true,
        None => {}
    };

    //Get sent boosts from db for returning
    match dbif::get_payments_from_db(&_ctx.database_file_path, index, boostcount, old, true) {
        Ok(streams) => {
            let json_doc_raw = serde_json::to_string_pretty(&streams).unwrap();
            let json_doc: String = strip::strip_tags(&json_doc_raw);

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .body(format!("{}", json_doc).into())
                .unwrap();
        }
        Err(e) => {
            eprintln!("** Error getting sent boosts: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("** Error getting sent boosts.").into())
                .unwrap();
        }
    }
}

pub async fn api_v1_reply_options(_ctx: Context) -> Response {
    return options_response("POST, OPTIONS".to_string())
}

pub async fn api_v1_reply(_ctx: Context) -> Response {
    let post_vars = get_post_params(_ctx.req).await;

    if post_vars.get("index").unwrap_or(&"".to_string()).is_empty() { // none or empty
        return client_error_response("** No index specified.".to_string());
    }

    if post_vars.get("sats").unwrap_or(&"".to_string()).is_empty() { // none or empty
        return client_error_response("** No sats specified.".to_string());
    }

    let lightning = match connect_to_lnd(_ctx.helipad_config.clone()).await {
        Some(lndconn) => lndconn,
        None => {
            return server_error_response("** Error connecting to LND.".to_string())
        }
    };

    let index = post_vars.get("index").unwrap().parse().unwrap();
    let sats = post_vars.get("sats").unwrap().parse().unwrap();
    let sender = match post_vars.get("sender") {
        Some(name) => name,
        None => "Anonymous"
    };
    let message = match post_vars.get("message") {
        Some(msg) => msg,
        None => ""
    };

    let boost = dbif::get_single_boost_from_db(&_ctx.helipad_config.database_file_path, index).unwrap();
    let tlv = boost.parse_tlv().unwrap();

    let reply_address = tlv["reply_address"].as_str();
    let mut custom_key = tlv["reply_custom_key"].as_u64();
    let mut custom_value = tlv["reply_custom_value"].as_str();

    let mut pub_key: &str = match reply_address {
        Some(addr) => addr,
        None => {
            return client_error_response("** No reply_address found in boost".to_string());
        }
    };

    let ln_info: LnAddressResponse;

    if pub_key.contains("@") { // pub_key is actually a lightning address
        ln_info = match resolve_lightning_address(pub_key).await {
            Ok(addy) => addy,
            Err(e) => {
                return server_error_response(format!("** Unable to resolve lightning address: {}", e).to_string());
            }
        };

        pub_key = ln_info.pubkey.as_str();

        if ln_info.custom_data.len() > 0 {
            let ckey_u64 = match ln_info.custom_data[0].custom_key.parse::<u64>() {
                Ok(val) => val,
                Err(_) => {
                    return server_error_response("** Unable to parse lightning address custom key".to_string());
                }
            };

            custom_key = Some(ckey_u64);
            custom_value = Some(ln_info.custom_data[0].custom_value.as_str());
        }
    }

    if custom_key.is_some() && custom_value.is_none() {
        return client_error_response("** No reply_custom_value found in boost".to_string());
    }

    let reply_tlv = json!({
        "app_name": "Helipad",
        "app_version": _ctx.state.version,
        "podcast": &tlv["podcast"],
        "episode": &tlv["episode"],
        "sender_name": sender,
        "message": message,
        "action": "boost",
        "value_msat_total": sats * 1000,
    });

    match send_boost(lightning, pub_key, custom_key, custom_value, sats, reply_tlv.clone()).await {
        Ok(payment) => {
            let custom_value_string = custom_value.map(|value| value.to_string());
            let payment_hash = HEXLOWER.encode(&payment.payment_hash);

            let mut sent_boost = dbif::SentBoostRecord {
                pubkey: pub_key.to_string(),
                custom_key: custom_key,
                custom_value: custom_value_string,
                sender: sender.to_string(),
                message: message.to_string(),
                podcast: tlv["podcast"].as_str().unwrap_or_default().to_string(),
                episode: tlv["episode"].as_str().unwrap_or_default().to_string(),
                total_amt_msat: sats * 1000,
                total_fees_msat: -1,
                payment_hash: payment_hash.clone(),
                reply_boost_index: Some(index),
                tlv: reply_tlv.to_string(),
            };

            if let Some(route) = payment.payment_route.clone() {
                sent_boost.total_amt_msat = route.total_amt_msat;
                sent_boost.total_fees_msat = route.total_fees_msat;
            }

            dbif::add_sent_boost_to_db(&_ctx.helipad_config.database_file_path, sent_boost).unwrap();

            println!("Payment: {:#?}", payment);

            let js = json!({
                "success": payment.payment_error.is_empty(),
                "message": payment.payment_error
            });

            if let Some(route) = payment.payment_route {
                println!("** Boost sent: pub_key={}, custom_key={}, custom_value={}, total_amt_msat={}, total_fees_msat={}, payment_hash={}",
                    pub_key,
                    custom_key.unwrap_or_default(),
                    custom_value.unwrap_or_default(),
                    route.total_amt_msat,
                    route.total_fees_msat,
                    payment_hash,
                );
            }
            else {
                eprintln!("** Failed to send boost: pub_key={}, custom_key={}, custom_value={}, error={}",
                    pub_key,
                    custom_key.unwrap_or_default(),
                    custom_value.unwrap_or_default(),
                    payment.payment_error
                );
            }

            return json_response(js);
        },
        Err(e) => {
            eprintln!("** Error sending boost: {}", e);
            return server_error_response(format!("** Error sending boost: {}", e))
        }
    }
}

pub async fn api_v1_node_alias_options(_ctx: Context) -> Response {
    return options_response("GET, OPTIONS".to_string())
}

pub async fn api_v1_node_alias(_ctx: Context) -> Response {
    let query_vars = get_query_params(_ctx.req);

    let pub_key = query_vars.get("pubkey").unwrap_or(&"".to_string()).to_string();

    if pub_key.is_empty() { // none or empty
        return client_error_response("** No pubkey specified.".to_string());
    }

    let mut lightning = match connect_to_lnd(_ctx.helipad_config.clone()).await {
        Some(lndconn) => lndconn,
        None => {
            return server_error_response("** Error connecting to LND.".to_string())
        }
    };

    let info = match lnd::Lnd::get_node_info(&mut lightning, pub_key, false).await {
        Ok(ninfo) => ninfo,
        Err(e) => {
            eprintln!("** Error getting node info: {}", e);
            return server_error_response("** Error getting node info".to_string())
        }
    };

    if info.node.is_none() {
        return json_response("");
    }

    return json_response(info.node.unwrap().alias);
}


//CSV export - max is 200 for now so the csv content can be built in memory
pub async fn csv_export_boosts(_ctx: Context) -> Response {
    //Get query parameters
    let params: HashMap<String, String> = _ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    //Parameter - index (unsigned int)
    let index: u64;
    match params.get("index") {
        Some(supplied_index) => {
            index = match supplied_index.parse::<u64>() {
                Ok(index) => {
                    println!("** Supplied index from call: [{}]", index);
                    index
                }
                Err(_) => {
                    eprintln!("** Error getting boosts: 'index' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting boosts: 'index' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Parameter - boostcount (unsigned int)
    let boostcount: u64;
    match params.get("count") {
        Some(bcount) => {
            boostcount = match bcount.parse::<u64>() {
                Ok(boostcount) => {
                    println!("** Supplied boostcount from call: [{}]", boostcount);
                    boostcount
                }
                Err(_) => {
                    eprintln!("** Error getting boosts: 'count' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting boosts: 'count' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Was the "old" flag used?
    let mut old = false;
    match params.get("old") {
        Some(_) => old = true,
        None => {}
    };

    //Was a stop index given?
    let mut endex: u64 = 0;
    match params.get("end") {
        Some(endexnum) => {
            endex = match endexnum.parse::<u64>() {
                Ok(endex) => {
                    println!("** Supplied endex from call: [{}]", endex);
                    endex
                }
                Err(_) => {
                    eprintln!("** Error getting boosts: 'endex' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'endex' parameter must be an integer.").into())
                        .unwrap();
                }
            };
        }
        None => {}
    };

    //Get the boosts from db for returning
    match dbif::get_boosts_from_db(&_ctx.helipad_config.database_file_path, index, boostcount, old, false) {
        Ok(boosts) => {
            let mut csv = String::new();

            //CSV column name header
            csv.push_str(format!("count,index,time,value_msat,value_sat,value_msat_total,value_sat_total,action,sender,app,message,podcast,episode,remote_podcast,remote_episode\n").as_str());

            //Iterate the boost set
            let mut count: u64 = 1;
            for boost in boosts {
                //Parse out a friendly date
                let dt = NaiveDateTime::from_timestamp(boost.time, 0);
                let boost_time = dt.format("%e %b %Y %H:%M:%S UTC").to_string();

                //Translate to sats
                let mut value_sat = 0;
                if boost.value_msat > 1000 {
                    value_sat = boost.value_msat / 1000;
                }
                let mut value_sat_total = 0;
                if boost.value_msat_total > 1000 {
                    value_sat_total = boost.value_msat_total / 1000;
                }

                //The main export data formatting
                csv.push_str(
                    format!(
                        "{},{},\"{}\",{},{},{},{},{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                        count,
                        boost.index,
                        boost_time,
                        boost.value_msat,
                        value_sat,
                        boost.value_msat_total,
                        value_sat_total,
                        boost.action,
                        BoostRecord::escape_for_csv(boost.sender),
                        BoostRecord::escape_for_csv(boost.app),
                        BoostRecord::escape_for_csv(boost.message),
                        BoostRecord::escape_for_csv(boost.podcast),
                        BoostRecord::escape_for_csv(boost.episode),
                        BoostRecord::escape_for_csv(boost.remote_podcast.unwrap_or("".to_string())),
                        BoostRecord::escape_for_csv(boost.remote_episode.unwrap_or("".to_string()))
                    ).as_str()
                );

                //Keep count
                count += 1;

                //If an exit point was given then bail when it's reached
                if (old && boost.index <= endex) || (!old && boost.index >= endex) {
                    break;
                }
            }

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .header("Content-type", "text/plain; charset=utf-8")
                .header("Content-Disposition", "attachment; filename=\"boosts.csv\"")
                .body(format!("{}", csv).into())
                .unwrap();
        }
        Err(e) => {
            eprintln!("** Error getting boosts: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("** Error getting boosts.").into())
                .unwrap();
        }
    }
}