use rusqlite::{Connection, params, Error::QueryReturnedNoRows};
use std::error::Error;
use serde::{Deserialize, Serialize};
use crate::{connect_to_database, HydraError};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SettingsRecord {
    pub show_received_sats: bool,
    pub show_split_percentage: bool,
    pub hide_boosts: bool,
    pub hide_boosts_below: Option<u64>,
    pub play_pew: bool,
    pub custom_pew_file: Option<String>,
    pub resolve_nostr_refs: bool,
    pub show_hosted_wallet_ids: bool,
    pub show_lightning_invoices: bool,
    pub fetch_metadata: bool,
    pub tts_enabled: bool,
    pub tts_rate: f32,
    pub tts_pitch: f32,
    pub tts_volume: f32,
    pub tts_on_boost: bool,
    pub tts_on_stream: bool,
    pub tts_on_payment: bool,
    pub tts_min_sats: Option<u64>,
    pub tts_script: Option<String>,
    pub tts_script_without_message: Option<String>,
    pub tts_voice: Option<String>,
    pub tts_provider: Option<String>, // "browser" or "amazon_polly"
    pub tts_aws_region: Option<String>,
    pub tts_aws_access_key_id: Option<String>,
    pub tts_aws_secret_access_key: Option<String>,
    pub tts_aws_voice_id: Option<String>, // e.g., "Joanna", "Matthew", "Ivy"
}

pub fn create_settings_table(conn: &Connection) -> Result<bool, Box<dyn Error>> {
    //Create the settings table
    match conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
             idx integer primary key autoincrement,
             show_received_sats integer not null,
             show_split_percentage integer not null,
             hide_boosts integer not null,
             hide_boosts_below integer,
             play_pew integer not null,
             custom_pew_file text
         )",
        [],
    ) {
        Ok(_) => {
            println!("Settings table is ready.");
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError("Failed to create database settings table.".into())))
        }
    }

    if conn.execute("ALTER TABLE settings ADD COLUMN resolve_nostr_refs integer DEFAULT 0", []).is_ok() {
        println!("Nostr refs setting added.");
    }

    if conn.execute("ALTER TABLE settings ADD COLUMN show_hosted_wallet_ids integer DEFAULT 0", []).is_ok() {
        println!("Hosted wallet id setting added.");
    }

    if conn.execute("ALTER TABLE settings ADD COLUMN show_lightning_invoices integer DEFAULT 1", []).is_ok() {
        println!("Show lightning invoices setting added.");
    }

    if conn.execute("ALTER TABLE settings ADD COLUMN fetch_metadata integer DEFAULT 1", []).is_ok() {
        println!("Fetch metadata setting added.");
    }

    // TTS settings
    if conn.execute(
        "ALTER TABLE settings
        ADD COLUMN tts_enabled integer DEFAULT 0,
        ADD COLUMN tts_rate real DEFAULT 1.0,
        ADD COLUMN tts_pitch real DEFAULT 1.0,
        ADD COLUMN tts_volume real DEFAULT 1.0,
        ADD COLUMN tts_on_boost integer DEFAULT 1,
        ADD COLUMN tts_on_stream integer DEFAULT 1,
        ADD COLUMN tts_on_payment integer DEFAULT 1,
        ADD COLUMN tts_min_sats integer,
        ADD COLUMN tts_script text,
        ADD COLUMN tts_script_without_message text,
        ADD COLUMN tts_voice text,
        ADD COLUMN tts_provider text,
        ADD COLUMN tts_aws_region text,
        ADD COLUMN tts_aws_access_key_id text,
        ADD COLUMN tts_aws_secret_access_key text,
        ADD COLUMN tts_aws_voice_id text
        ", []
    ).is_ok() {
        println!("TTS settings added.");
    }

    Ok(true)
}

pub fn load_settings_from_db(filepath: &String) -> Result<SettingsRecord, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let mut stmt = conn.prepare(
        r#"SELECT
             show_received_sats,
             show_split_percentage,
             hide_boosts,
             hide_boosts_below,
             play_pew,
             custom_pew_file,
             resolve_nostr_refs,
             show_hosted_wallet_ids,
             show_lightning_invoices,
             fetch_metadata,
             tts_enabled,
             tts_rate,
             tts_pitch,
             tts_volume,
             tts_on_boost,
             tts_on_stream,
             tts_on_payment,
             tts_min_sats,
             tts_script,
             tts_script_without_message,
             tts_voice,
             tts_provider,
             tts_aws_region,
             tts_aws_access_key_id,
             tts_aws_secret_access_key,
             tts_aws_voice_id
        FROM
            settings
        WHERE
            idx = 1
        "#
    )?;

    let result = stmt.query_row([], |row| {
        Ok(SettingsRecord {
            show_received_sats: row.get(0)?,
            show_split_percentage: row.get(1)?,
            hide_boosts: row.get(2)?,
            hide_boosts_below: row.get(3).ok(),
            play_pew: row.get(4)?,
            custom_pew_file: row.get(5).ok(),
            resolve_nostr_refs: row.get(6)?,
            show_hosted_wallet_ids: row.get(7)?,
            show_lightning_invoices: row.get(8)?,
            fetch_metadata: row.get(9).unwrap_or(true),
            tts_enabled: row.get(10).unwrap_or(false),
            tts_rate: row.get(11).unwrap_or(1.0),
            tts_pitch: row.get(12).unwrap_or(1.0),
            tts_volume: row.get(13).unwrap_or(1.0),
            tts_on_boost: row.get(14).unwrap_or(true),
            tts_on_stream: row.get(15).unwrap_or(true),
            tts_on_payment: row.get(16).unwrap_or(true),
            tts_min_sats: row.get(17).ok(),
            tts_script: row.get(18).ok(),
            tts_script_without_message: row.get(19).ok(),
            tts_voice: row.get(20).ok(),
            tts_provider: row.get(21).ok(),
            tts_aws_region: row.get(22).ok(),
            tts_aws_access_key_id: row.get(23).ok(),
            tts_aws_secret_access_key: row.get(24).ok(),
            tts_aws_voice_id: row.get(25).ok(),
        })
    });

    match result {
        Ok(s) => Ok(s),
        Err(QueryReturnedNoRows) => Ok(SettingsRecord {
            show_received_sats: false,
            show_split_percentage: false,
            hide_boosts: false,
            hide_boosts_below: None,
            play_pew: true,
            custom_pew_file: None,
            resolve_nostr_refs: false,
            show_hosted_wallet_ids: false,
            show_lightning_invoices: true,
            fetch_metadata: true,
            tts_enabled: false,
            tts_rate: 1.0,
            tts_pitch: 1.0,
            tts_volume: 1.0,
            tts_on_boost: true,
            tts_on_stream: true,
            tts_on_payment: true,
            tts_min_sats: None,
            tts_script: None,
            tts_script_without_message: None,
            tts_voice: None,
            tts_provider: None,
            tts_aws_region: None,
            tts_aws_access_key_id: None,
            tts_aws_secret_access_key: None,
            tts_aws_voice_id: None,
        }),
        Err(e) => Err(Box::new(e)),
    }
}

pub fn save_settings_to_db(filepath: &String, settings: &SettingsRecord) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    match conn.execute(
        r#"INSERT INTO settings (
            idx,
            show_received_sats,
            show_split_percentage,
            hide_boosts,
            hide_boosts_below,
            play_pew,
            custom_pew_file,
            resolve_nostr_refs,
            show_hosted_wallet_ids,
            show_lightning_invoices,
            fetch_metadata,
            tts_enabled,
            tts_rate,
            tts_pitch,
            tts_volume,
            tts_on_boost,
            tts_on_stream,
            tts_on_payment,
            tts_min_sats,
            tts_script,
            tts_script_without_message,
            tts_voice,
            tts_provider,
            tts_aws_region,
            tts_aws_access_key_id,
            tts_aws_secret_access_key,
            tts_aws_voice_id
        )
        VALUES
            (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)
        ON CONFLICT(idx) DO UPDATE SET
            show_received_sats = excluded.show_received_sats,
            show_split_percentage = excluded.show_split_percentage,
            hide_boosts = excluded.hide_boosts,
            hide_boosts_below = excluded.hide_boosts_below,
            play_pew = excluded.play_pew,
            custom_pew_file = excluded.custom_pew_file,
            resolve_nostr_refs = excluded.resolve_nostr_refs,
            show_hosted_wallet_ids = excluded.show_hosted_wallet_ids,
            show_lightning_invoices = excluded.show_lightning_invoices,
            fetch_metadata = excluded.fetch_metadata,
            tts_enabled = excluded.tts_enabled,
            tts_rate = excluded.tts_rate,
            tts_pitch = excluded.tts_pitch,
            tts_volume = excluded.tts_volume,
            tts_on_boost = excluded.tts_on_boost,
            tts_on_stream = excluded.tts_on_stream,
            tts_on_payment = excluded.tts_on_payment,
            tts_min_sats = excluded.tts_min_sats,
            tts_script = excluded.tts_script,
            tts_script_without_message = excluded.tts_script_without_message,
            tts_voice = excluded.tts_voice,
            tts_provider = excluded.tts_provider,
            tts_aws_region = excluded.tts_aws_region,
            tts_aws_access_key_id = excluded.tts_aws_access_key_id,
            tts_aws_secret_access_key = excluded.tts_aws_secret_access_key,
            tts_aws_voice_id = excluded.tts_aws_voice_id
        "#,
        params![
            settings.show_received_sats,
            settings.show_split_percentage,
            settings.hide_boosts,
            settings.hide_boosts_below,
            settings.play_pew,
            settings.custom_pew_file,
            settings.resolve_nostr_refs,
            settings.show_hosted_wallet_ids,
            settings.show_lightning_invoices,
            settings.fetch_metadata,
            settings.tts_enabled,
            settings.tts_rate,
            settings.tts_pitch,
            settings.tts_volume,
            settings.tts_on_boost,
            settings.tts_on_stream,
            settings.tts_on_payment,
            settings.tts_min_sats,
            settings.tts_script,
            settings.tts_script_without_message,
            settings.tts_voice,
            settings.tts_provider,
            settings.tts_aws_region,
            settings.tts_aws_access_key_id,
            settings.tts_aws_secret_access_key,
            settings.tts_aws_voice_id,
        ]
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            Err(Box::new(HydraError("Failed to save settings".into())))
        }
    }
}