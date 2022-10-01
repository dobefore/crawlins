//! 以下是模块文档
//!
//! from website `https://www.zdic.net/ (汉典网)`
//! 查询成语后输出页面是以这样格式显示在浏览器输入栏：
//! `https://www.zdic.net/hans/%E6%AC%B2%E7%9B%96%E5%BC%A5%E5%BD%B0 （欲盖弥彰转码）`
//!
//! So,in order to get the page,we could use the url by prefixing uri `https://www.zdic.net/hans/`
//! with chengyu entry which has already been transformed to uri code.
//!
//! # Analysis of entry web page.
//! ## 拼音部分：
//! ```
//! <span class="dicpy">yù gài mí zhāng</span>  
//! ```
//!
//! ## 解释部分的元素块
//! ```
//! <div class="content definitions cnr">
//!  <h3>欲盖弥彰</h3><p>【解释】盖：遮掩；弥：更加；彰：明显。想掩盖坏事的真相，结果反而更明显地暴露出来。</p><p>【出处】《左传·昭公三十一年》：“或求名而不得，或欲盖而名章，惩不义也。”</p><p>【示例】与其～，倒不如自己先认了。 ◎闻一多《画展》</p><p>【反义词】相得益彰</p><p>【语法】紧缩式；作谓语、宾语、定语；含贬义</p>                    <div class="div copyright"> © 汉典 </div>
//! </div>
//! ```

use std::{collections::HashMap, fs, io::Write, sync::Arc};

use crate::{
    error::Result,
    utils::{group_by_range, request_text, selector_parse_doc, selector_parse_frac},
};
use serde::{Deserialize, Serialize};

static PINYIN: &str = r#"span[class="dicpy"]"#;
static DEFINITIONS: &str = r#"div[class="content definitions cnr"]"#;
static PREFIX_URL: &str = "https://www.zdic.net/hans/";
use futures::future::join_all;
use tokio::sync::Mutex;
pub async fn handle() {
    // read entries from file
    let mut f = fs::read_to_string("entry.txt").unwrap();

    let f = f.lines().collect::<Vec<_>>();
    let mut entries = vec![];
    for e in f {
        entries.push(e.to_string());
    }
    let mut map = HashMap::new();

    let cyc = query_batch(&entries).await.unwrap();
    for i in cyc.chengyucol().as_ref() {
        map.insert(i.entry().to_string(), i.to_owned());
    }

    // after crawl done,write to map
    let s = serde_json::to_string(&map).unwrap();
    fs::write("entries.txt", s).unwrap();
}
/// It means handian chengyu.
///
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct HanDianCY {
    pinyin: String,
    entry: String,
    /// 出处
    source: Option<String>,
    /// 解释
    meaning: Option<String>,
    example: Option<String>,
    /// 近义词
    synonym: Option<String>,
    /// 反义词
    antonym: Option<String>,
}

impl HanDianCY {
    fn new(entry: String) -> Self {
        Self {
            entry,
            ..Default::default()
        }
    }

    fn set_pinyin(&mut self, pinyin: String) {
        self.pinyin = pinyin;
    }

    fn set_source(&mut self, source: Option<String>) {
        self.source = source;
    }

    fn set_meaning(&mut self, meaning: Option<String>) {
        self.meaning = meaning;
    }

    fn set_example(&mut self, example: Option<String>) {
        self.example = example;
    }

    /// parse a block string to fields of [`HanDianCY`].
    ///
    /// set its fields if that field is present in block str.
    fn set_definitions(&mut self, block_str: Vec<String>) -> &mut Self {
        for b in block_str {
            if b.contains("解释") {
                let r = b.replace("【解释】", "");
                self.set_meaning(Some(r));
            }
            if b.contains("【出处】") {
                let r = b.replace("【出处】", "");
                self.set_source(Some(r));
            }
            if b.contains("【示例】") {
                let r = b.replace("【示例】", "");
                self.set_example(Some(r));
            }
            if b.contains("【反义词】") {
                let r = b.replace("【反义词】", "");
                self.set_antonym(Some(r));
            }
            if b.contains("【近义词】") {
                let r = b.replace("【近义词】", "");
                self.set_synonym(Some(r));
            }
        }

        self
    }

    fn set_synonym(&mut self, synonym: Option<String>) {
        self.synonym = synonym;
    }

    fn set_antonym(&mut self, antonym: Option<String>) {
        self.antonym = antonym;
    }

    pub fn pinyin(&self) -> &str {
        self.pinyin.as_ref()
    }

    pub fn entry(&self) -> &str {
        self.entry.as_ref()
    }

    pub fn source(&self) -> Option<&String> {
        self.source.as_ref()
    }

    pub fn meaning(&self) -> Option<&String> {
        self.meaning.as_ref()
    }

    pub fn synonym(&self) -> Option<&String> {
        self.synonym.as_ref()
    }

    pub fn antonym(&self) -> Option<&String> {
        self.antonym.as_ref()
    }
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct HanDianCYCollection {
    chengyucol: Vec<HanDianCY>,
}

impl HanDianCYCollection {
    fn new(chengyucol: Vec<HanDianCY>) -> Self {
        Self { chengyucol }
    }

    pub fn chengyucol(&self) -> &[HanDianCY] {
        self.chengyucol.as_ref()
    }
}
/// query only one entry
///
/// # example
/// ```
///     use tokio::runtime::Runtime;
/// let rt=Runtime::new();
/// let r=rt.unwrap().block_on(query_one("火中取栗")).unwrap();
///
/// result:
///
/// {"pinyin":"huǒ zhōng qǔ lì","entry":"火中取栗","source":"十七世纪法国寓言诗人拉·封丹的寓言《猴子与猫》载：猴子骗猫取火中栗子，栗子让猴子吃了，猫却把脚上的毛烧掉了。","meaning":"偷取炉中烤熟的栗子。比喻受人利用，冒险出力却一
/// 无所得。","example":"我们目前自顾不暇，郑成功不来就是天主保佑了，我们还好去惹他么。我们不能为别人～。 ◎郭沫若《
/// 郑成功》第五章","synonym":"代人受过、为人作嫁","antonym":"坐享其成"}
/// ```
pub async fn query_one(entry: &str) -> Result<String> {
    let url = format!("{}{}", PREFIX_URL, entry);
    let mut cy = HanDianCY::new(entry.into());
    let html = request_text(&url).await?;
    let py = parse_pinyin(&html, PINYIN)?;
    cy.set_pinyin(py);
    let def_block = parse_definttion_block(&html, DEFINITIONS)?;
    cy.set_definitions(def_block);
    let json = serde_json::to_string(&cy)?;
    Ok(json)
}
/// query more than entry
/// dump json string to file
pub async fn query_batch_and_dump(entries: &[String], fpath: &str) -> Result<()> {
    let json = query_batch_json(entries).await?;
    tokio::fs::write(fpath, json).await?;

    Ok(())
}
/// query more than entry
///
/// return value in json string
pub async fn query_batch_json(entries: &[String]) -> Result<String> {
    let cyc = query_batch(entries).await?;
    let json = serde_json::to_string(&cyc)?;
    Ok(json)
}
/// query more than entry
///
/// return [`HanDianCYCollection`]
///# example
/// ```  
/// use tokio::runtime::Runtime;
/// let rt=Runtime::new();
/// let r=rt.unwrap().block_on(query_batch(&["火中取栗".to_string(),"无法无天".to_string()])).unwrap();
/// ```
pub async fn query_batch(entries: &[String]) -> Result<HanDianCYCollection> {
    // use for loop instead of iter map
    let urls = entries
        .iter()
        .map(|e| -> Result<(String, String)> {
            Ok((e.to_string(), to_url_code(format!("{}{}", PREFIX_URL, e))?))
        })
        .collect::<Vec<_>>();
    let mut v = vec![];
    let cys = Arc::new(Mutex::new(vec![]));
    let file = Arc::new(Mutex::new(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("error.txt")?,
    ));
    for u in urls {
        match u {
            Ok(url) => v.push(url),
            Err(e) => return Err(crate::CrawlInsError::UrlTransform(e.to_string())),
        }
    }
    let group = group_by_range(v, 15);

    let (tx, mut rx) = tokio::sync::mpsc::channel(15);
    tokio::spawn(async move {
        for url_group in group {
            if tx.send(url_group).await.is_err() {
                println!("receiver dropped");
                return;
            }
        }
    });

    while let Some(ug) = rx.recv().await {
        let mut handles = vec![];
        for (entry, url) in ug {
            let cys = cys.clone();
            let file = file.clone();
            let mut limit = 0;

            handles.push(tokio::spawn(async move {
                loop {
                    limit += 1;
                    if limit >= 3 {
                        break;
                    }
                    if let Err(e) = run_task(cys.clone(), entry.clone(), url.clone()).await {
                        file.lock().await.write(format!("{}\n",entry).as_bytes()).unwrap();
                        println!("{}", e);
                        break;
                    } else {
                        break;
                    }
                }
            }));
        }
        join_all(handles).await;
    }

    let mut temp = vec![];
    cys.lock()
        .await
        .iter()
        .for_each(|e| temp.push(e.to_owned()));
    let cyc = HanDianCYCollection::new(temp);

    Ok(cyc)
}
async fn run_task(cys: Arc<Mutex<Vec<HanDianCY>>>, entry: String, url: String) -> Result<()> {
    println!("{entry}");
    let url=to_url_code(format!("{}{}",PREFIX_URL,entry.replace("，", "")))?;
    let mut cy = HanDianCY::new(entry);
    let html = request_text(&url).await?;
    let py = parse_pinyin(&html, PINYIN)?;
    cy.set_pinyin(py);
    let def_block = parse_definttion_block(&html, DEFINITIONS)?;
    cy.set_definitions(def_block);
    cys.lock().await.push(cy);
    Ok(())
}
///
/// return pinyin str.
///
/// # example
/// ```
///  let html=r#"<span class="dicpy">yù gài mí zhāng</span>  "#;
/// let py=parse_pinyin(html, PINYIN).unwrap();
/// ```
///
/// # Errors
///
/// This function will return an error if pinyin element not found.
fn parse_pinyin(html: &str, selector: &str) -> Result<String> {
    let (document, selector) = selector_parse_doc(html, selector)?;
    let mut elements = document.select(&selector);
    // assume there is only one ele
    let py = if let Some(e) = elements.next() {
        let mut py = e
            .text()
            .into_iter()
            .map(<str as ToString>::to_string)
            .collect::<Vec<_>>();
        py.remove(0)
    } else {
        return Err(crate::CrawlInsError::ParseHtmlSelector(
            "pinyin item not found".into(),
        ));
    };
    Ok(py)
}
/// convert raw string to url code
///
/// # example
/// ```
/// let raw="http://a.b.c/我们";
/// to_url_code(raw);
///
/// output:
/// http://a.b.c/%E6%88%91%E4%BB%AC
/// ```
fn to_url_code<U: reqwest::IntoUrl>(raw_str: U) -> Result<String> {
    Ok(raw_str.into_url()?.to_string())
}
/// parse html to get a collections of p elements which contain all sorts of definitions.
/// then get text from these p s.
///
/// return a string contains all sorts of fields of [`HanDianCY`]
///
/// # return sample
/// ```
/// 【解释】盖：遮掩；弥：更加；彰：明显。想掩盖坏事的真相，结果反而更明显地暴露出来。
/// 【出处】《左传·昭公三十一年》：“或求名而不得，或欲盖而名章，惩不义也。”
/// 【示例】与其～，倒不如自己先认了。 ◎闻一多《画展》
/// 【反义词】相得益彰
/// 【语法】紧缩式；作谓语、宾语、定语；含贬义
///```
///
/// # Errors
///
/// This function will return an error if .
fn parse_definttion_block(html: &str, selector: &str) -> Result<Vec<String>> {
    let (document, selector) = selector_parse_doc(html, selector)?;
    let mut element = document.select(&selector);
    let mut block = vec![];
    if let Some(e) = element.next() {
        let (html, sel) = selector_parse_frac(&e.html(), "p")?;
        let elements = html.select(&sel);
        for ele in elements {
            let item = ele
                .text()
                .into_iter()
                .map(<str as ToString>::to_string)
                .collect::<Vec<_>>()
                .remove(0);
            block.push(item);
        }
    } else {
        return Err(crate::CrawlInsError::ParseHtmlSelector(
            "div element not found".into(),
        ));
    }

    Ok(block)
}
// use genanki_rs::{Field, Model, Template, Error,Deck,Note};
// async fn make_deck() -> Result<()> {
//     let my_model = Model::new(
//         1607392319,
//         "Simple Model",
//         vec![Field::new("Question"), Field::new("Answer")],
//         vec![Template::new("Card 1")
//             .qfmt("{{Question}}")
//             .afmt(r#"{{FrontSide}}<hr id="answer">{{Answer}}"#)],
//     );
//     let mut my_deck = Deck::new(
//         2059400110,
//         "Country Capitals",
//         "Deck for studying country capitals",
//     );
//     let my_note = Note::new(my_model, vec!["Capital of Argentina", "Buenos Aires"]).unwrap();
//     my_deck.add_note(my_note);
//     my_deck.write_to_file("chengyu.apkg").unwrap();
//     let entries=["无法无天".to_string(),"火中取栗".to_string()];
//     let cyc=query_batch(&entries).await?;
// let col=cyc.chengyucol();

// for h in col {

// }
//     Ok(())
// }
/// test whether web page can be gotten successfully or not.
#[test]
fn test_get_page() {
    use tokio::runtime::Runtime;
    // use std::io::Write;
    use crate::utils::request_text;

    let rt = Runtime::new().unwrap();
    let link = "https://www.zdic.net/hans/%E6%AC%B2%E7%9B%96%E5%BC%A5%E5%BD%B0";
    let s = rt.block_on(request_text(link)).unwrap();
    assert!(s.contains("content definitions cnr"));

    // let  f=std::fs::File::create("./t.html");
    // f.expect("msg").write_all(s.as_bytes()).unwrap();
}
#[test]
fn test_parse_pinyin() {
    let html = r#"<span class="dicpy">yù gài mí zhāng</span>  "#;
    let py = parse_pinyin(html, PINYIN).unwrap();

    assert_eq!("yù gài mí zhāng".to_string(), py)
}

#[test]
fn test_query_one() {
    use tokio::runtime::Runtime;
    let rt = Runtime::new();
    let r = rt.unwrap().block_on(query_one("总而言之")).unwrap();
    println!("{}", r)
}

#[test]
fn test_handle() {
    use tokio::runtime::Runtime;
    let rt = Runtime::new();
    rt.unwrap().block_on(handle());
}
