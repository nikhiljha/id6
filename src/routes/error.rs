use actix_web::{HttpResponse, error};
use derive_more::{Display, Error};
use actix_web::http::{StatusCode, header};
use actix_web::dev::HttpResponseBuilder;

pub(crate) fn error_page(msg: &str) -> VerificationError {
    return VerificationError::Message { field: msg.to_string() };
}

#[derive(Debug, Display, Error)]
pub(crate) enum VerificationError {
    #[display(fmt = "Unable to link your accounts: {}", field)]
    Message { field: String },
}

impl error::ResponseError for VerificationError {
    fn status_code(&self) -> StatusCode {
        match *self {
            VerificationError::Message { .. } => StatusCode::BAD_REQUEST,
        }
    }
    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code())
            .set_header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(self.to_string())
    }
}
