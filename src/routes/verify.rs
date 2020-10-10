use actix_web::{HttpRequest, web, HttpResponse, Error};
use crate::{AppState, DatabaseContainer};
use askama::Template;
use serenity::model::id::{ChannelId, GuildId, UserId, RoleId};
use tokio_postgres::row::RowIndex;
use serenity::model::guild::Member;

#[derive(Template)]
#[template(path = "verify.html")]
struct VerifyTemplate<'a> {
    service: &'a str,
    token: &'a str,
    discord_name: &'a str,
    external_name: &'a str,
}

#[derive(Template)]
#[template(path = "success.html")]
struct SuccessTemplate {}

fn get_ocf_username(req: &HttpRequest) -> Option<&str> {
    req.headers().get("X-Auth-Username")?.to_str().ok()
}

pub(crate) async fn verify_post(req: HttpRequest, data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let token = req.match_info().get("token").unwrap_or("invalid");
    let ser_d = data.discord_ctx.data.read().await;

    let pg_client = match ser_d.get::<DatabaseContainer>() {
        Some(v) => v,
        None => {
            return Ok(HttpResponse::InternalServerError().content_type("text/html").body("<p>Database connection failure.</p>"));
        }
    };

    let row = match pg_client.query_one(
        "SELECT uuid, name, token, completed FROM verifications WHERE token = $1 AND completed = $2",
        &[&token, &false],
    ).await {
        Ok(v) => v,
        Err(_e) => {
            return Ok(HttpResponse::InternalServerError().content_type("text/html").body("<p>Invalid token.</p>"));
        }
    };

    let uuid: String = row.get("uuid");
    let uuid_64 = uuid.parse::<u64>().expect("UUID in database is not a number!");
    let mut member: Member = match GuildId(746867420072771687).member(&data.discord_ctx.http, uuid_64).await {
        Ok(v) => v,
        Err(_e) => {
            return Ok(HttpResponse::InternalServerError().content_type("text/html").body("<p>Member no longer in channel.</p>"));
        }
    };

    match member.add_role(&data.discord_ctx.http, RoleId(764325476977868860)).await {
        Ok(v) => v,
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().content_type("text/html").body("<p>Unable to add role!</p>"));
        }
    };

    match pg_client.execute(
        "UPDATE verifications SET completed = TRUE WHERE token = $1",
        &[&token],
    ).await {
        Ok(v) => (),
        Err(_e) => {
            println!("WARN: Unable to invalidate the verification.");
        }
    };

    if let Err(why) = ChannelId(746867420072771690).send_message(&data.discord_ctx, |m| m.embed(|e| {
        e.title("New user verified!");
        e.field(
            "Name",
            member.user.name,
            true,
        );
        e
    })).await {
        eprintln!("Error sending message: {:?}", why);
    };
    let s = SuccessTemplate{}.render().expect("Unable to render success template.");
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

pub(crate) async fn verify_get(req: HttpRequest, data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let user = get_ocf_username(&req).expect("Unable to retrieve OCF username.");
    let token = req.match_info().get("token").unwrap_or("invalid");
    let ser_d = data.discord_ctx.data.read().await;

    let pg_client = match ser_d.get::<DatabaseContainer>() {
        Some(v) => v,
        None => {
            return Ok(HttpResponse::InternalServerError().content_type("text/html").body("<p>Database connection failure.</p>"));
        }
    };

    let row = match pg_client.query_one(
        "SELECT uuid, name, token, completed FROM verifications WHERE token = $1 AND completed = $2",
        &[&token, &false],
    ).await {
        Ok(v) => v,
        Err(_e) => {
            return Ok(HttpResponse::InternalServerError().content_type("text/html").body("<p>Invalid token.</p>"));
        }
    };

    let s = VerifyTemplate {
        service: "OCF",
        token: row.get("token"),
        discord_name: row.get("name"),
        external_name: user,
    }
        .render()
        .unwrap();
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}