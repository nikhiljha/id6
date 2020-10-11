use actix_web::{Result};

use serenity::client::{Client};
use serenity::static_assertions::_core::sync::atomic::AtomicBool;

use serde_derive::Deserialize;

use std::{env};
use std::fs::File;
use std::io::Read;

use crate::bot::Handler;
use crate::bot::DatabaseContainer;
use native_tls::{Certificate, TlsConnector};
use postgres_native_tls::MakeTlsConnector;

mod routes;
mod db;
mod templates;
mod bot;

#[derive(Deserialize, Clone)]
struct Config {
    admin_channel: u64,
    verify_channel: u64,
    guild_id: u64,
    role_id: u64,
    msg_id: u64,
    base_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // configuration
    let config_file_path = env::var("CONFIG_FILE_PATH")
        .expect("Could not read CONFIG_FILE_PATH from environment, is it set?");
    let mut config_file = File::open(config_file_path)
        .expect("Unable to open config file.");
    let mut config_file_contents = String::new();
    config_file.read_to_string(&mut config_file_contents)
        .expect("Unable to read config file.");
    let config: Config = toml::from_str(&config_file_contents)
        .expect("Unable to parse config file.");

    // discord bot
    // TODO: Add commands back in (right now it's using a dummy StandardFramework).
    let token = env::var("DISCORD_TOKEN")
        .expect("Could not read DISCORD_TOKEN from environment, is it set?");
    let mut client = Client::new(token)
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
            config
        })
        .framework(serenity::framework::standard::StandardFramework::new())
        .await
        .expect("Unable to create Discord client.");

    // TLS for postgres
    let connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()?;
    let connector = MakeTlsConnector::new(connector);

    // postgres database
    let connstring = env::var("POSTGRES_CONN")
        .expect("Could not read POSTGRES_CONN from environment, is it set?");
    let (pg, connection) =
        tokio_postgres::connect(&connstring, connector)
            .await
            .expect("Unable to connect to configured database.");
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    db::run_migrations(&pg).await;

    // start discord bot
    // note: webserver is started inside the discord bot (bot.rs)
    {
        let mut data = client.data.write().await;
        data.insert::<DatabaseContainer>(pg);
    }
    tokio::spawn(async move {
        if let Err(why) = client.start().await {
            println!("Discord client error: {:?}", why);
        }
    }).await.expect("Unable to start Discord bot.");

    Ok(())
}
