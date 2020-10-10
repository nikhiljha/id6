use askama::Template;

#[derive(Template)]
#[template(path = "verify.html")]
pub(crate) struct VerifyTemplate<'a> {
    pub(crate) service: &'a str,
    pub(crate) token: &'a str,
    pub(crate) discord_name: &'a str,
    pub(crate) external_name: &'a str,
}

#[derive(Template)]
#[template(path = "success.html")]
pub(crate) struct SuccessTemplate {}
