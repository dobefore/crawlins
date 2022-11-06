//! this is a collection of many instances of crawl.
//! use features to enable each of functions.
mod error;
#[cfg(feature = "chengyu")]
pub mod handian;
mod utils;
#[cfg(feature = "vocabulary")]
pub mod vocabulary;
pub mod webster;
pub use error::{Error, Result};
