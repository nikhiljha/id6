use crate::{Config, routes};
use std::sync::Arc;
use serenity::static_assertions::_core::sync::atomic::{Ordering, AtomicBool};
use serenity::client::{Context, EventHandler};
use serenity::model::id::{GuildId, RoleId};
use serenity::async_trait;
use std::thread;
use actix_web::{HttpServer, web, App};
use actix::System;
use actix_cors::Cors;
use actix_files::Files;
use serenity::model::guild::Member;
use serenity::model::channel::Reaction;
use serenity::prelude::TypeMapKey;
use uuid::Uuid;

pub(crate) struct Handler {
    pub(crate) is_loop_running: AtomicBool,
    pub(crate) config: Config,
}

pub(crate) struct AppState {
    pub(crate) discord_ctx: Arc<Context>,
    pub(crate) config: Config,
}

pub(crate) struct DatabaseContainer;

impl TypeMapKey for DatabaseContainer {
    type Value = tokio_postgres::Client;
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
                            config: cfg1.clone(),
                        })
                        .wrap(Cors::new().allowed_origin(&cfg1.base_url).allowed_methods(vec!["GET", "POST"]).finish())
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
        Handler::send_verification_dm(&self, &ctx, &_guild_id, &member).await;
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        let gid: GuildId = reaction.guild_id.expect("unable to get guild id");
        let member: Member = gid.member(
            &ctx,
            reaction.user(&ctx).await.expect("unable to get user who reacted"),
        ).await.expect("unable to get member who reacted");
        if reaction.message_id == self.config.msg_id {
            for x in member.roles(&ctx).await.expect("unable to get member roles") {
                if x.id.0 == RoleId(self.config.role_id).0 {
                    return;
                }
            }
            Handler::send_verification_dm(
                &self,
                &ctx,
                &gid,
                &member,
            ).await;
        }
    }
}

impl Handler {
    async fn send_verification_dm(&self, ctx: &Context, _guild_id: &GuildId, member: &Member) {
        let data = ctx.data.read().await;
        let pg_client = match data.get::<DatabaseContainer>() {
            Some(v) => v,
            None => {
                // TODO: Send message w/ failure notification.
                return ();
            }
        };

        // TODO: Check if the member is already verified.
        let token = Uuid::new_v4();
        pg_client.execute(
            "INSERT INTO verifications (uuid, name, guild, token, completed) VALUES ($1, $2, $3, $4, $5)",
            &[&member.user.id.to_string(), &member.user.name, &_guild_id.to_string(), &token.to_string(), &false],
        ).await.expect("Unable to write to database.");
        let url = format!("{}/verify/{}", self.config.base_url, token.to_string());

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
