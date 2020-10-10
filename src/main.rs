use actix_files::Files;
use actix_web::{web, App, HttpServer, Result};
use actix_cors::Cors;

use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};

use serde_derive::Deserialize;

use std::{env, thread};
use serenity::model::id::{GuildId};
use serenity::model::guild::Member;
use std::sync::Arc;
use serenity::static_assertions::_core::sync::atomic::{Ordering, AtomicBool};
use actix::System;
use serenity::prelude::TypeMapKey;

use uuid::Uuid;
use serenity::model::channel::Reaction;
use std::fs::File;
use std::io::Read;

mod routes;
mod db;

#[derive(Deserialize, Clone)]
struct Config {
    channel_id: u64,
    guild_id: u64,
    role_id: u64,
    msg_id: u64,
    base_url: String,
}

struct Handler {
    is_loop_running: AtomicBool,
    config: Config
}

struct AppState {
    discord_ctx: Arc<Context>,
    config: Config
}

#[async_trait]
impl EventHandler for Handler {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        println!("Cache built successfully!");
        let ctx = Arc::new(ctx);
        if !self.is_loop_running.load(Ordering::Relaxed) {

            // setup actix
            let ctx1 = Arc::clone(&ctx);
            let cfg1 = self.config.clone();
            thread::spawn(move || {
                let sys = System::new("http-server");

                HttpServer::new(move || {
                    App::new()
                        .data(AppState {
                            discord_ctx: Arc::clone(&ctx1),
                            config: cfg1.clone()
                        })
                        .wrap(Cors::new().allowed_origin("http://localhost:8080").allowed_methods(vec!["GET", "POST"]).finish())
                        .route("/verify/{token}", web::get().to(routes::verify_get))
                        .route("/verify/{token}", web::post().to(routes::verify_post))
                        .service(Files::new("/", "./static/").index_file("index.html"))
                })
                    .bind("0.0.0.0:8080")
                    .expect("Failed to bind to port 8080.")
                    .run();

                sys.run()
            });

            // Now that the loop is running, we set the bool to true
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }

    async fn guild_member_addition(&self, ctx: Context, _guild_id: GuildId, member: Member) {
        Handler::send_verification_dm(&ctx, &_guild_id, &member).await;
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        let gid: GuildId = reaction.guild_id.expect("unable to get guild id");
        let member: Member = gid.member(
            &ctx,
            reaction.user(&ctx).await.expect("unable to get user who reacted")
        ).await.expect("unable to get member who reacted");
        if reaction.message_id == 1 {
            Handler::send_verification_dm(
                &ctx,
                &gid,
                &member
            ).await;
        }
    }
}


struct DatabaseContainer;

impl TypeMapKey for DatabaseContainer {
    type Value = tokio_postgres::Client;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_file_path = env::var("CONFIG_FILE_PATH")
        .expect("Could not read CONFIG_FILE_PATH from environment, is it set?");
    let mut config_file = File::open(config_file_path)?;
    let mut config_file_contents = String::new();
    config_file.read_to_string(&mut config_file_contents)?;
    let config: Config = toml::from_str(&config_file_contents).expect("TOML parsing failed...");

    println!("{}", config.channel_id);

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

    // setup postgres
    let (pg, connection) =
        tokio_postgres::connect("host=localhost user=postgres password=hello", tokio_postgres::NoTls)
            .await
            .expect("Unable to connect to configured database.");
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    db::run_migrations(&pg);

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

impl Handler {
    async fn send_verification_dm(ctx: &Context, _guild_id: &GuildId, member: &Member) {
        let data = ctx.data.read().await;
        let pg_client = match data.get::<DatabaseContainer>() {
            Some(v) => v,
            None => {
                // TODO: Send message w/ failure notification.
                return ();
            },
        };

        // TODO: Check if the member is already verified.
        let token = Uuid::new_v4();
        pg_client.execute(
            "INSERT INTO verifications (uuid, name, guild, token, completed) VALUES ($1, $2, $3, $4, $5)",
            &[&member.user.id.to_string(), &member.user.name, &_guild_id.to_string(), &token.to_string(), &false]
        ).await.expect("Unable to write to database.");
        let url = format!("https://discord.ocf.berkeley.edu/verify/{}", token.to_string());

        let dm = member.user.dm(&ctx, |m| {
            m.content(format!("Welcome to the OCF Discord Server! To see every channel, you'll need to \
            verify your Discord account with your OCF account at the link below. \n{}", url))
        }).await;

        match dm {
            Ok(_) => {
                println!("Sent verification link via DM. {}", url);
            }
            Err(why) => {
                println!("Error DMing verification link: {:?}", why);
            }
        };
    }
}
