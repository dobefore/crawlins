//! this is a collection of many instances of crawl.
mod error;
pub mod handian;
mod utils;
pub use error::{CrawlInsError, Result};
pub fn add(left: usize, right: usize) -> usize {
    left + right
}
