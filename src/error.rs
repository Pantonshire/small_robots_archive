use std::borrow::Cow;
use std::error;
use std::fmt;

use actix_web::{HttpRequest, HttpResponse};
use actix_web::Responder;
use actix_web::error::ResponseError;
use actix_web::http::StatusCode;
use log::log;

use crate::page;
use crate::respond::MarkupResponse;

pub type SiteReportResult<T> = Result<T, SiteReportError>;

/// Wrapper struct for [SiteError] with an additional diagnostic message.
#[derive(Debug)]
pub struct SiteReportError {
    pub message: Cow<'static, str>,
    pub err: SiteError,
}

impl SiteReportError {
    pub fn new<S>(message: S, err: SiteError) -> Self where S: Into<Cow<'static, str>> {
        Self {
            message: message.into(),
            err,
        }
    }
}

impl fmt::Display for SiteReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.message, self.err)
    }
}

impl ResponseError for SiteReportError {
    fn status_code(&self) -> StatusCode {
        self.err.status_code()
    }

    fn error_response(&self) -> HttpResponse {
        log!(self.err.log_level(), "{}", self);

        let status = self.status_code();
        MarkupResponse::new(page::error_page(status), status).into()
    }
}

impl Responder for SiteReportError {
    fn respond_to(self, _: &HttpRequest) -> HttpResponse {
        self.error_response()
    }
}

#[derive(Debug)]
pub enum SiteError {
    BadRequest,
    NotFound,
    DatabaseError(Box<sqlx::Error>),
}

impl fmt::Display for SiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SiteError::BadRequest => write!(f, "bad request"),
            SiteError::NotFound => write!(f, "resource not found"),
            SiteError::DatabaseError(err) => write!(f, "database error: {}", err),
        }
    }
}

impl error::Error for SiteError {}

impl SiteError {
    pub fn report<S>(self, message: S) -> SiteReportError where S: Into<Cow<'static, str>> {
        SiteReportError::new(message, self)
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn log_level(&self) -> log::Level {
        match self {
            Self::BadRequest => log::Level::Warn,
            Self::NotFound => log::Level::Warn,
            Self::DatabaseError(_) => log::Level::Error,
        }
    }
}

impl From<sqlx::Error> for SiteError {
    fn from(err: sqlx::Error) -> Self {
        Self::DatabaseError(Box::new(err))
    }
}

pub trait IntoReport {
    fn into_report<S>(self, message: S) -> SiteReportError where S: Into<Cow<'static, str>>;
}

impl<E> IntoReport for E where E: Into<SiteError> {
    fn into_report<S>(self, message: S) -> SiteReportError where S: Into<Cow<'static, str>> {
        self.into().report(message)
    }
}
