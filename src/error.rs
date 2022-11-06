use std::{num, result};
pub type Result<T> = result::Result<T, Error>;
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("ParseInt error: {0}")]
    ParseInt(#[from] num::ParseIntError),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("JsonParse error {0}")]
    Tokio(#[from] tokio::task::JoinError),
    #[error("JsonParse error {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Download error {0}")]
    Download(String),
    #[error("ParseHtmlSelector error {0}")]
    ParseHtmlSelector(String),
    #[error("UrlTransform error {0}")]
    UrlTransform(String),
}
