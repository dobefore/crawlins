//! include utils about how to handle requests and responses.
use crate::error::{CrawlInsError, Result};
use scraper::{Html, Selector};
/// request html page text
pub(crate) async fn request_text(link: &str) -> Result<String> {
    let  pc="Mozilla/5.0 (Windows NT 6.1) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/41.0.2228.0 Safari/537.36";
    let text = reqwest::ClientBuilder::new()
        .user_agent(pc)
        .build()?
        .get(link)
        .send()
        .await?
        .text()
        .await?;
    Ok(text)
}
/// split a vec of values into multi-smaller vec,and put them into a vec .
///
/// # example
/// ```
/// let range=5;
/// let v=(0..10).collect::<Vec<_>>();
/// // [[0, 1, 2, 3, 4], [5, 6, 7, 8, 9]]
/// println!("{:?}",group_by_range(v, range))
///```
pub(crate) fn group_by_range<T>(mut v: Vec<T>, range: u8) -> Vec<Vec<T>> {
    let mut g = vec![];
    loop {
        if v.is_empty() {
            break;
        }
        if v.len() < range.into() && !v.is_empty() {
            g.push(v);
            break;
        }
        let cs = v.drain(0..range as usize).collect::<Vec<_>>();
        g.push(cs);
    }
    g
}
pub(crate) fn selector_parse_frac(html: &str, selector: &str) -> Result<(Html, Selector)> {
    let fragment = Html::parse_fragment(html);
    match Selector::parse(selector) {
        Ok(s) => Ok((fragment, s)),
        Err(_) => Err(CrawlInsError::ParseHtmlSelector(format!(
            "parse {} element error",
            selector
        ))),
    }
}
pub(crate) fn selector_parse_doc(html: &str, selector: &str) -> Result<(Html, Selector)> {
    let fragment = Html::parse_document(html);
    match Selector::parse(selector) {
        Ok(s) => Ok((fragment, s)),
        Err(_) => Err(CrawlInsError::ParseHtmlSelector(format!(
            "parse {} element error",
            selector
        ))),
    }
}

#[test]
fn test_group_by_range() {
    let range = 5;
    let v = (0..10).collect::<Vec<_>>();
    // [[0, 1, 2, 3, 4], [5, 6, 7, 8, 9]]
    println!("{:?}", group_by_range(v, range))
}
