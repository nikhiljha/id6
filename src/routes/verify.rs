use actix_web::{HttpRequest, web, HttpResponse, Error};
use serenity::model::id::{ChannelId, GuildId, RoleId};
use serenity::model::guild::Member;
use askama::Template;

use crate::bot::{AppState, DatabaseContainer};
use crate::routes::{error_page};
use crate::db::invalidate_verification;
use crate::templates::{VerifyTemplate, SuccessTemplate};

fn get_ocf_username(req: &HttpRequest) -> Option<&str> {
    req.headers().get("X-Auth-Username")?.to_str().ok()
}

pub(crate) async fn verify_post(req: HttpRequest, data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let user = get_ocf_username(&req).expect("Unable to retrieve OCF username.");
    let token = req.match_info().get("token").unwrap_or("invalid");
    let ser_d = data.discord_ctx.data.read().await;

    let pg_client = ser_d.get::<DatabaseContainer>()
        .ok_or_else(|| 0)
        .map_err(|_e| error_page("Database connection error."))?;


    let row = pg_client.query_one(
        "SELECT uuid, name, token, completed FROM verifications WHERE token = $1 AND completed = $2",
        &[&token, &false],
    )
        .await
        .map_err(|_e| error_page("Invalid token."))?;

    let uuid: String = row.get("uuid");
    let uuid_64 = uuid.parse::<u64>().expect("UUID in database is not a number!");
    let mut member: Member = GuildId(data.config.guild_id).member(&data.discord_ctx.http, uuid_64)
        .await
        .map_err(|_e| error_page("Unable to get member, did you leave?"))?;

    member.add_role(&data.discord_ctx.http, RoleId(data.config.role_id))
        .await
        .map_err(|_e| error_page("Unable to add role."))?;

    match invalidate_verification(&pg_client, &token).await {
        Ok(_v) => (),
        Err(_e) => {
            println!("WARN: Unable to invalidate the verification.");
        }
    };

    let name: String = row.get("name");
    if let Err(why) = ChannelId(data.config.admin_channel)
        .send_message(&data.discord_ctx, |m|
            m.content(format!(
                "Discord user {} (UUID: {}) has been linked to OCF account {}.",
                name, uuid, user
            ))).await {
        println!("WARN: Unable to log verification: {:?}", why);
    };

    let s = SuccessTemplate {}
        .render()
        .map_err(|_e| error_page("Unable to render success page."))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

pub(crate) async fn verify_get(req: HttpRequest, data: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let user = get_ocf_username(&req).expect("Unable to retrieve OCF username.");
    let token = req.match_info().get("token").unwrap_or("invalid");
    let ser_d = data.discord_ctx.data.read().await;

    let pg_client = ser_d.get::<DatabaseContainer>()
        .ok_or_else(|| 0)
        .map_err(|_e| error_page("Database connection error."))?;

    let row = pg_client.query_one(
        "SELECT uuid, name, token, completed FROM verifications WHERE token = $1 AND completed = $2",
        &[&token, &false],
    )
        .await
        .map_err(|_e| error_page("Invalid token."))?;

    let s = VerifyTemplate {
        service: "OCF",
        token: row.get("token"),
        discord_name: row.get("name"),
        external_name: user,
    }.render().expect("Unable to render VerifyTemplate.");
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}