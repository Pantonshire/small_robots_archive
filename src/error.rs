use std::error;
use std::fmt;

use actix_web::{HttpRequest, HttpResponse};
use actix_web::Responder;
use actix_web::error::ResponseError;
use actix_web::http::{StatusCode, header};

pub type SiteResult<T> = Result<T, SiteError>;

#[derive(Debug)]
pub enum SiteError {
    NotFound,
    DatabaseError(Box<sqlx::Error>),
}

impl fmt::Display for SiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SiteError::NotFound => write!(f, "resource not found"),
            SiteError::DatabaseError(err) => write!(f, "database error: {}", err),
        }
    }
}

impl error::Error for SiteError {}

impl ResponseError for SiteError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();

        let mut res = HttpResponse::new(status_code);

        res.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/plain; charset=utf-8"),
        );

        let message = format!(
            "{} {}",
            status_code.as_str(),
            status_code.canonical_reason().unwrap_or("unknown error")
        );

        res.set_body(actix_web::body::AnyBody::from(message))
    }
}

impl Responder for SiteError {
    fn respond_to(self, _: &HttpRequest) -> HttpResponse {
        self.error_response()
    }
}

impl From<sqlx::Error> for SiteError {
    fn from(err: sqlx::Error) -> Self {
        Self::DatabaseError(Box::new(err))
    }
}
