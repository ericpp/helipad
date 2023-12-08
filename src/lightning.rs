use data_encoding::HEXLOWER;
use lnd::lnrpc::lnrpc::{SendRequest, SendResponse};
use serde_json::Value;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::error::Error;
use rand::RngCore;

// TLV keys (see https://github.com/satoshisstream/satoshis.stream/blob/main/TLV_registry.md)
pub const TLV_PODCASTING20: u64 = 7629169;
pub const TLV_WALLET_KEY: u64 = 696969;
pub const TLV_WALLET_ID: u64 = 112111100;
pub const TLV_HIVE_ACCOUNT: u64 = 818818;
pub const TLV_KEYSEND: u64 = 5482373484;


#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LnAddressResponse {
    status: String,
    tag: String,
    pubkey: String,
    custom_data: Vec<LnAddressCustomData>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LnAddressCustomData {
    custom_key: String,
    custom_value: String,
}

#[derive(Debug)]
pub struct LnAddressError(String);

impl std::fmt::Display for LnAddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "There is an error: {}", self.0)
    }
}

impl std::error::Error for LnAddressError {}

pub async fn connect_to_lnd(node_address: String, cert_path: String, macaroon_path: String) -> Option<lnd::Lnd> {
    let cert: Vec<u8>;
    match fs::read(cert_path.clone()) {
        Ok(cert_content) => {
            // println!(" - Success.");
            cert = cert_content;
        }
        Err(_) => {
            eprintln!("Cannot find a valid tls.cert file");
            return None;
        }
    }

    let macaroon: Vec<u8>;
    match fs::read(macaroon_path.clone()) {
        Ok(macaroon_content) => {
            // println!(" - Success.");
            macaroon = macaroon_content;
        }
        Err(_) => {
            eprintln!("Cannot find a valid admin.macaroon file");
            return None;
        }
    }

    //Make the connection to LND
    let lightning = lnd::Lnd::connect_with_macaroon(node_address.clone(), &cert, &macaroon).await;

    if lightning.is_err() {
        println!("Could not connect to: [{}] using tls: [{}] and macaroon: [{}]", node_address, cert_path, macaroon_path);
        eprintln!("{:#?}", lightning.err());
        return None;
    }

    return lightning.ok();
}

pub async fn resolve_lightning_address(address: &str) -> Result<LnAddressResponse, Box<dyn Error>> {
    if !address.contains('@') {
        return Err(Box::new(LnAddressError("Invalid lightning address".to_string())));
    }

    if !email_address::EmailAddress::is_valid(address) {
        return Err(Box::new(LnAddressError("Invalid lightning address".to_string())));
    }

    let parts: Vec<&str> = address.split('@').collect();

    if parts.len() != 2 {
        return Err(Box::new(LnAddressError("Invalid lightning address".to_string())));
    }

    let url = format!("https://{}/.well-known/keysend/{}", parts[1], parts[0]);
    println!("Resolving Lightning Address {} through {}", address, url);

    let response = reqwest::get(url.clone()).await?.text().await?;
    let data: LnAddressResponse = serde_json::from_str(&response)?;

    if data.custom_data.len() > 0 {
        println!("Lightning Address {}: pub_key={}, custom_key={}, custom_value={}",
            address,
            data.pubkey,
            data.custom_data[0].custom_key,
            data.custom_data[0].custom_value,
        );
    }
    else {
        println!("Lightning Address {}: pub_key={}", address, data.pubkey);
    }

    return Ok(data);
}

pub async fn send_boost(mut lightning: lnd::Lnd, pub_key: &str, custom_key: Option<u64>, custom_value: Option<&str>, sats: i64, tlv: Value) -> Result<SendResponse, Box<dyn Error>> {
    // thanks to BrianOfLondon and Mostro for keysend details:
    // https://peakd.com/@brianoflondon/lightning-keysend-is-strange-and-how-to-send-keysend-payment-in-lightning-with-the-lnd-rest-api-via-python
    // https://github.com/MostroP2P/mostro/blob/52a4f86c3942c26bd42dc55f1e53db5da9f7542b/src/lightning/mod.rs#L18

    let mut real_pub_key = pub_key;
    let mut real_custom_key = custom_key;
    let mut real_custom_value = custom_value;

    let ln_info: LnAddressResponse;

    if pub_key.contains("@") { // pub_key is actually a lightning address
        ln_info = resolve_lightning_address(pub_key).await?;

        real_pub_key = ln_info.pubkey.as_str();

        if ln_info.custom_data.len() > 0 {
            let ckey_u64 = ln_info.custom_data[0].custom_key.parse::<u64>()?;
            real_custom_key = Some(ckey_u64);
            real_custom_value = Some(ln_info.custom_data[0].custom_value.as_str());
        }
    }

    // convert pub key hash to raw bytes
    let raw_pub_key = HEXLOWER.decode(real_pub_key.as_bytes()).unwrap();

    // generate 32 random bytes for pre_image
    let mut pre_image = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut pre_image);

    // and convert to sha256 hash
    let mut hasher = Sha256::new();
    hasher.update(pre_image);
    let payment_hash = hasher.finalize();

    // TLV custom records
    // https://github.com/satoshisstream/satoshis.stream/blob/main/TLV_registry.md
    let mut dest_custom_records = HashMap::new();
    let tlv_json = serde_json::to_string_pretty(&tlv).unwrap();

    dest_custom_records.insert(TLV_KEYSEND, pre_image.to_vec());
    dest_custom_records.insert(TLV_PODCASTING20, tlv_json.as_bytes().to_vec());

    if real_custom_key.is_some() && real_custom_value.is_some() {
        dest_custom_records.insert(real_custom_key.unwrap(), real_custom_value.unwrap().as_bytes().to_vec());
    }

    // assemble the lnd payment
    let req = SendRequest {
        dest: raw_pub_key.clone(),
        amt: sats,
        payment_hash: payment_hash.to_vec(),
        dest_custom_records: dest_custom_records,
        ..Default::default()
    };

    // send payment
    match lnd::Lnd::send_payment_sync(&mut lightning, req).await {
        Ok(payment) => Ok(payment),
        Err(e) => Err(Box::new(e))
    }
}
