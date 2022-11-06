//! https://www.vocabulary.com/dictionary
//! # Inspect the web page
//! ```
//! word
//! <div class="word-area">
//!<p class="short"><i>Happy</i> is a feeling of joy,....</p>
//!<p class="long"><i>Happy</i> hails from the Middle English word <i>hap</i>, meaning... </p>
//! </div>
//!
//! <div class="word-definitions">
//!
//! <li class="sense pos_a ... sord1" id="s101621">
//! <div class="definition">
//! <div title="adjective" name="s101621" class="pos-icon">adjective</div>
//!  marked by good fortune</div>
//!
//! <div class="example">&#8220;a <strong>happy</strong> outcome&#8221;</div>
//! </li>
//!
//! <li class="sense pos_a ... sord1" id="s101621">
//! <div class="definition">
//! <div title="adjective" name="s102164" class="pos-icon">adjective</div>
//! enjoying or showing or marked by joy or pleasure</div>
//!
//! <div class="example">&#8220;a <strong>happy</strong> smile&#8221;</div>
//! <div class="example">&#8220;spent many <strong>happy</strong> days on the beach&#8221;</div>
//! ...
//! </li>
//! </div>
//! ```
//! As we can see from above,we will get short and long word area,definition and examples.
//! As for word area,we parse `p` element.
//! As for definitions,we first parse `div-word-definitions`-->li-->div<definition>,div<example>`
#![allow(dead_code)]

use crate::{
    error::Result,
    utils::{self, group_by_range, remove_escape_code, request_text, to_url_code},
};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, fs, io::Write, sync::Arc};
use tokio::sync::Mutex;

static WORD_AREA: &str = r#"div[class="word-area"]"#;
static LONG: &str = r#"P[class="long"]"#;
static SHORT: &str = r#"P[class="short"]"#;
static DEFINITIONS: &str = r#"div[class="word-definitions"]"#;
static DEFINITION_BLCOK: &str = "li";
static DEFINITION: &str = r#"div[class="definition"]"#;
static EXAMPLE: &str = r#"div[class="example"]"#;
static SYNONYM: &str = r#"a[class="word"]"#;
static PREFIX_URL: &str = "https://www.vocabulary.com/dictionary/";
static ERROR_FILE: &str = "error.txt";
async fn query_batch_dump(words: &[String], fpath: &str) -> Result<()> {
    let ret = query_batch(words).await?;
    let js = serde_json::to_string(&ret)?;
    std::fs::write(fpath, js)?;
    Ok(())
}
pub async fn query_batch(words: &[String]) -> Result<Vec<Vocabulary>> {
    let urls = words
        .iter()
        .map(|e| {
            (
                e.to_string(),
                to_url_code(format!("{}{}", PREFIX_URL, e)).ok(),
            )
        })
        .collect::<Vec<_>>();
    let vocabs = Arc::new(Mutex::new(vec![]));
    let error_file = Arc::new(Mutex::new(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(ERROR_FILE)?,
    ));
    let group = group_by_range(urls, 15);

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
        for (word, url) in ug {
            let error_file = error_file.clone();
            let vocabs = vocabs.clone();
            let mut limit = 0;

            handles.push(tokio::spawn(async move {
                loop {
                    limit += 1;
                    if limit >= 3 {
                        error_file
                            .lock()
                            .await
                            .write_all(format!("{}\n", word).as_bytes())
                            .unwrap();

                        break;
                    }
                    if let Err(e) =
                        run_task(vocabs.clone(), url.as_ref().unwrap(), &word.clone()).await
                    {
                        println!("{}", e);
                    } else {
                        break;
                    }
                }
            }));
        }
        join_all(handles).await;
    }

    let mut temp = vec![];
    vocabs
        .lock()
        .await
        .iter()
        .for_each(|e| temp.push(e.to_owned()));

    Ok(temp)
}
async fn run_task(vocabs: Arc<Mutex<Vec<Vocabulary>>>, url: &str, word: &str) -> Result<()> {
    let mut vocab = Vocabulary::new(word.into());
    let html = request_text(url).await?;
    let area = parse_word_area(&html)?;
    let defs = parse_definitions(&html)?;

    vocab.set_word_area(area);
    vocab.set_definitions(defs);

    vocabs.lock().await.push(vocab);

    Ok(())
}
/// # sample html
/// ```
/// <div class="word-area">
///<p class="short"><i>Happy</i> is a feeling of joy,....</p>
///<p class="long"><i>Happy</i> hails from the Middle English word <i>hap</i>, meaning... </p>
/// </div>
/// ```
fn parse_word_area(html: &str) -> Result<Area> {
    let (html, sel) = utils::selector_parse_doc(html, WORD_AREA)?;
    let elements = html.select(&sel).next();
    let area = if let Some(element) = elements {
        let html = element.html();
        let (htmll, sell) = utils::selector_parse_frac(&html, LONG)?;
        let (htmls, sels) = utils::selector_parse_frac(&html, SHORT)?;
        let long = htmll.select(&sell).next();
        let short = htmls.select(&sels).next();
        let long_area = long.map(|l| l.text().collect::<Vec<_>>().join("").trim().to_string());
        let short_area = short.map(|s| s.text().collect::<Vec<_>>().join("").trim().to_string());
        let mut area = Area::new();
        area.set_long(long_area);
        area.set_short(short_area);
        area
    } else {
        Area::new()
    };
    Ok(area)
}
///  html code sample
/// ```
/// <dl class="instances">
/// <span class="detail">synonyms:</span>
/// <span><a href="/dictionary/felicitous" class="word">felicitous</a></span>
/// </dl>
/// <dl class="instances">
/// <span class="detail"></span>
/// <dd >
/// <a href="/dictionary/fortunate" class="word">fortunate</a>
/// <div class="definition">having unexpected good fortune</div>
/// </dd>
/// </dl>
/// ```
///
/// We need to parse element `span` whose class="word"
fn parse_synonym(html: &str) -> Result<Option<Vec<String>>> {
    let (html, sel) = utils::selector_parse_doc(html, SYNONYM)?;
    let elements = html.select(&sel);
    let ret = elements
        .into_iter()
        .map(|e| e.text().collect::<Vec<_>>().join(""))
        .collect::<Vec<_>>();

    Ok(if ret.is_empty() { None } else { Some(ret) })
}
/// # html code sample
/// ```
///  <div class="word-definitions">
///
/// <li class="sense pos_a ... sord1" id="s101621">
/// <div class="definition">
/// <div title="adjective" name="s101621" class="pos-icon">adjective</div>
///  marked by good fortune</div>
///
/// <div class="example">&#8220;a <strong>happy</strong> outcome&#8221;</div>
/// </li>
///
/// <li class="sense pos_a ... sord1" id="s101621">
/// <div class="definition">
/// <div title="adjective" name="s102164" class="pos-icon">adjective</div>
/// enjoying or showing or marked by joy or pleasure</div>
///
/// <div class="example">&#8220;a <strong>happy</strong> smile&#8221;</div>
/// <div class="example">&#8220;spent many <strong>happy</strong> days on the beach&#8221;</div>
///
/// <dl class="instances">
/// <span class="detail">synonyms:</span>
/// <span><a href="/dictionary/felicitous" class="word">felicitous</a></span>
/// </dl>
/// <dl class="instances">
/// <span class="detail"></span>
/// <dd >
/// <a href="/dictionary/fortunate" class="word">fortunate</a>
/// <div class="definition">having unexpected good fortune</div>
/// </dd>
/// </dl>
/// </li>
/// </div>
///
/// ```
fn parse_definitions(html: &str) -> Result<Vec<Definition>> {
    let mut defs = vec![];
    let (html, sel) = utils::selector_parse_doc(html, DEFINITIONS)?;
    let elements = html.select(&sel).next();
    if let Some(element) = elements {
        let html = element.html();
        let (html, sel) = utils::selector_parse_frac(&html, DEFINITION_BLCOK)?;
        let elements = html.select(&sel);
        //   definition block
        for e in elements {
            let mut def = Definition::new();
            let html = e.html();
            let (html_def, sel_def) = utils::selector_parse_frac(&html, DEFINITION)?;
            let element_def = html_def.select(&sel_def).next();
            let (html_exam, sel_exam) = utils::selector_parse_frac(&html, EXAMPLE)?;
            let elements_exam = html_exam.select(&sel_exam).collect::<Vec<_>>();

            let def_text = element_def
                .as_ref()
                .unwrap()
                .text()
                .map(remove_escape_code)
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            let examples = if !elements_exam.is_empty() {
                Some(
                    elements_exam
                        .iter()
                        .map(|e| {
                            e.text()
                                .map(remove_escape_code)
                                .collect::<Vec<_>>()
                                .join(" ")
                                .trim()
                                .to_string()
                        })
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            };
            let synonym = parse_synonym(&html)?;
            def.set_definition(def_text);
            def.set_examples(examples);
            def.set_synonym(synonym);

            defs.push(def);
        }
    }

    Ok(defs)
}
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Vocabulary {
    word: String,
    word_area: Area,
    definitions: Vec<Definition>,
}
impl Vocabulary {
    fn new(word: String) -> Self {
        Self {
            word,
            ..Default::default()
        }
    }

    pub fn set_word_area(&mut self, word_area: Area) {
        self.word_area = word_area;
    }

    pub fn set_definitions(&mut self, definitions: Vec<Definition>) {
        self.definitions = definitions;
    }
}
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]

pub struct Area {
    short: Option<String>,
    long: Option<String>,
}
impl Display for Area {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\n{}",
            self.short.as_ref().map_or("", |e| e.as_str()),
            self.long.as_ref().map_or("", |e| e.as_str())
        )
    }
}
impl Area {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_short(&mut self, short: Option<String>) {
        self.short = short;
    }

    pub fn set_long(&mut self, long: Option<String>) {
        self.long = long;
    }
}
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]

pub struct Definition {
    definition: String,
    synonym: Option<Vec<String>>,
    examples: Option<Vec<String>>,
}
impl Display for Definition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let example = if let Some(e) = self.examples.as_ref() {
            if !e.is_empty() {
                format!("Example: {}", e.join("\n"))
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        let synonym = if let Some(s) = self.synonym.as_ref() {
            let s = if s.len() >= 5 {
                s[..5].join(" ")
            } else {
                s.join(" ")
            };
            format!("synonym: {}", s)
        } else {
            String::new()
        };
        let exam_syno = if example.is_empty() && synonym.is_empty() {
            String::new()
        } else if example.is_empty() && !synonym.is_empty() {
            format!("{}", synonym)
        } else if !example.is_empty() && synonym.is_empty() {
            format!("{}", example)
        } else {
            format!("{}\n{}", example, synonym)
        };
        write!(f, "\nDefinition: {}\n{}", self.definition, exam_syno)
    }
}
impl Definition {
    fn new() -> Self {
        Self::default()
    }

    fn set_definition(&mut self, definition: String) {
        self.definition = definition;
    }

    fn set_synonym(&mut self, synonym: Option<Vec<String>>) {
        self.synonym = synonym;
    }

    pub fn set_examples(&mut self, examples: Option<Vec<String>>) {
        self.examples = examples;
    }
}

#[test]
fn test_parse_word_area() {
    let html = r#"<div class="word-area">
<p class="short"><i>Happy</i> is a feeling of joy,....</p>
<p class="long"><i>Happy</i> hails from the Middle English word <i>hap</i>, meaning... </p>
 </div>"#;
    let a = parse_word_area(html).unwrap();
    let mut area = Area::new();
    area.set_long(Some(
        "Happy hails from the Middle English word hap, meaning...".into(),
    ));
    area.set_short(Some("Happy is a feeling of joy,....".into()));
    assert_eq!(area, a)
}
pub fn task(file: &str) -> Result<()> {
    use genanki_rs::{Deck, Error, Field, Model, Note, Template};
    let v: Vec<Vocabulary> = serde_json::from_reader(fs::File::open(file)?)?;
    let my_model = Model::new(
        1607392319,
        "Simple Model",
        vec![
            Field::new("Word"),
            Field::new("Area"),
            Field::new("Definitions"),
        ],
        vec![Template::new("Card 1")
            .qfmt("{{Word}}")
            .afmt(r#"{{FrontSide}}<hr id="answer">{{Area}}<br/>{{Definitions}}"#)],
    );
    let mut my_deck = Deck::new(
        2059400110,
        "3000-frequently-used-words",
        "Deck for studying country capitals",
    );
    for e in v {
        println!("{}", e.word);
        let defs = e
            .definitions
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        let my_note = Note::new(
            my_model.clone(),
            vec![
                e.word.replace("\n", "<br>").as_str(),
                e.word_area.to_string().as_str(),
                defs.replace("\n", "<br>").as_str(),
            ],
        )
        .unwrap();
        my_deck.add_note(my_note);
    }
    my_deck.write_to_file("vocabulary.apkg").unwrap();
    Ok(())
}
#[test]
fn test_parse_definitions() {
    let html = r#"
 <div class="word-definitions">

<li class="sense pos_a ... sord1" id="s101621">
<div class="definition">
<div title="adjective" name="s101621" class="pos-icon">adjective</div>
 marked by good fortune</div>

<div class="example">&#8220;a <strong>happy</strong> outcome&#8221;</div>

<dl class="instances">
<span class="detail">synonyms:</span>
<span><a href="/dictionary/felicitous" class="word">felicitous</a></span>		
</dl>										
<dl class="instances">
<span class="detail"></span>
<dd >
<a href="/dictionary/fortunate" class="word">fortunate</a>
<div class="definition">having unexpected good fortune</div>
</dd>					
</dl>

</li>

<li class="sense pos_a ... sord1" id="s101621">
<div class="definition">
<div title="adjective" name="s102164" class="pos-icon">adjective</div>
enjoying or showing or marked by joy or pleasure</div>

<div class="example">&#8220;a <strong>happy</strong> smile&#8221;</div>
<div class="example">&#8220;spent many <strong>happy</strong> days on the beach&#8221;</div>
...
</li>
</div>


"#;
    let defs = parse_definitions(html).unwrap();

    let mut expected1 = Definition::new();
    let mut expected2 = Definition::new();
    let mut expected = vec![];
    expected1.set_definition("adjective  marked by good fortune".into());
    expected1.set_examples(Some(["“a happy outcome”".into()].to_vec()));
    expected1.set_synonym(Some(["felicitous".into(), "fortunate".into()].to_vec()));

    expected2.set_definition("adjective enjoying or showing or marked by joy or pleasure".into());
    expected2.set_examples(Some(
        [
            "“a happy smile”".into(),
            "“spent many happy days on the beach”".into(),
        ]
        .to_vec(),
    ));
    expected.push(expected1);
    expected.push(expected2);

    assert_eq!(defs, expected);
}

#[test]
fn test_parse_synonym() {
    let html = r#"
   <dl class="instances">
 <span class="detail">synonyms:</span>
 <span><a href="/dictionary/felicitous" class="word">felicitous</a></span>		
 </dl>										
 <dl class="instances">
 <span class="detail"></span>
 <dd >
 <a href="/dictionary/fortunate" class="word">fortunate</a>
 <div class="definition">having unexpected good fortune</div>
 </dd>					
 </dl>
    "#;
    let ret = parse_synonym(html).unwrap();
    println!("{:?}", ret)
}

#[test]
fn test_batch() {
    use tokio::runtime::Runtime;
    let words = ["absolutely".to_string(), "thrash".to_string()];
    let rt = Runtime::new().unwrap();
    let ret = rt.block_on(query_batch(&words)).unwrap();
    println!("{:?}", ret);
}
#[test]
fn test_t() {
    let url = "https://www.merriam-webster.com/dictionary/give";
    use tokio::runtime::Runtime;
    let rt = Runtime::new().unwrap();
    let t = rt.block_on(request_text(url)).unwrap();
    fs::write("give.html", t).unwrap();
    use utils;
    // let s= fs::read_to_string(".html").unwrap();
    // let (html, sel) = utils::selector_parse_doc(&s, r#"div[class="vg"]"#).unwrap();
    // let elements = html.select(&sel).next();
    // std::fs::write("a.html", elements.as_ref().unwrap().html()).unwrap();
}
