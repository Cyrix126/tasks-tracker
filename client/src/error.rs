use reqwest::header::{InvalidHeaderValue, ToStrError};
use thiserror::Error;
use url::ParseError;

/// thiserror Error struct, possibles Errors returned by this client library.
#[derive(Error, Debug)]
pub enum TaskClientError {
    #[error(transparent)]
    EncodeError(#[from] bincode::error::EncodeError),
    #[error(transparent)]
    DecodeError(#[from] bincode::error::DecodeError),
    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    #[error(transparent)]
    ErrorRequest(#[from] reqwest::Error),
    #[error("the conversion of the header value to str failed")]
    HeaderToStr(#[from] ToStrError),
    #[error("can't parse Content-Location Response Header to Url")]
    ContentLocationParse(#[from] ParseError),
    #[error("Header {0} is not present in Response")]
    HeaderNotFound(String),
}
