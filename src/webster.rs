//! website: https://www.merriam-webster.com/dictionary/happy
//!
#![allow(dead_code)]
use crate::error::Result;
use crate::utils::{
    group_by_range, request_text, selector_parse_doc, selector_parse_frac, to_url_code,
};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, fs, io::Write, sync::Arc};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
static ROOT_URL: &str = "https://www.merriam-webster.com";
static DICT: &str = "https://www.merriam-webster.com/dictionary/";
/// there may exist more than one block
/// id="dictionary-entry-{1}" 1-5
static DEFINITIONS: &str = r#"div[class="vg"]"#;
static DEFINITION_BLOCK: &str = r#"div[class="vg-sseq-entry-item"]"#;
static DEFINITION: &str = r#"span[class="dt "]"#;
static MEANING: &str = r#"span[class="dtText"]"#;
/// sometimes this is absent
static EXAMPLES: &str = r#"span[class="sub-content-thread"]"#;
/// means phrases associated only with word
static PHRASE_WORD: &str = r#"span[id="phrases"]"#;

static PHRASE_ITEM: &str = r#"span[class="drp"]"#;
static PHRASE_MEANING: &str = r#"div[class="dt "]"#;
// phrases
static PHRASE_BLOCK: &str = r#"div[class="related-phrases-list-container-xs"]"#;
/// directly call text() to get phrase text,href to get partial url
static PHRASE: &str = r#"a[class="pb-4 pr-4 d-block"]"#;
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Webster {
    word: String,
    definitions: Vec<Definition>,
    phrases: Option<Vec<Phrase>>,
}

impl Webster {
    fn new(word: String) -> Self {
        Self {
            word,
            ..Default::default()
        }
    }

    pub fn set_definitions(&mut self, definitions: Vec<Definition>) {
        self.definitions = definitions;
    }

    pub fn set_phrases(&mut self, phrases: Option<Vec<Phrase>>) {
        self.phrases = phrases;
    }
}
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Definition {
    definition: String,
    examples: Option<Vec<String>>,
}

impl Definition {
    fn new() -> Self {
        Self::default()
    }

    pub fn set_definition(&mut self, definition: String) {
        self.definition = definition;
    }

    pub fn set_examples(&mut self, examples: Option<Vec<String>>) {
        self.examples = examples;
    }
}
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Phrase {
    entry: String,
    definitions: Vec<String>,
}
impl Phrase {
    fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
struct T {
    item: String,
    phrases: String,
}

impl T {
    fn new(item: String) -> Self {
        Self {
            item,
            ..Default::default()
        }
    }

    fn set_phrases(&mut self, phrases: String) {
        self.phrases = phrases;
    }
}
async fn run_task(vocabs: Arc<Mutex<Vec<T>>>, url: &str, word: &str) -> Result<()> {
    let mut vocab = T::new(word.into());
    let html = request_text(url).await?;
    let phr = parse_phrase(&html)?;
    vocab.set_phrases(phr.map_or("".to_string(), |v| v.join("\n")));
    vocabs.lock().await.push(vocab);
    // let phrases=parse_phrase(&html)?;

    Ok(())
}
pub async fn gen_anki() -> Result<()> {
    let error_file = Arc::new(Mutex::new(tokio::fs::File::open("error.txt").await?));
    let vocabs = Arc::new(Mutex::new(Vec::new()));
    let words = fs::read_to_string("voc.txt")?
        .lines()
        .map(<str as ToString>::to_string)
        .collect::<Vec<_>>();
    let urls = words
        .iter()
        .map(|e| {
            (
                e.to_string(),
                to_url_code(format!(
                    "{}{}",
                    "https://www.merriam-webster.com/dictionary/", e
                ))
                .ok(),
            )
        })
        .collect::<Vec<_>>();
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
                            .await
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

    make_cards(vocabs.lock().await.to_vec());
    Ok(())
}

fn make_cards(cards: Vec<T>) {
    use genanki_rs::{Deck, Field, Model, Note, Template};
    let my_model = Model::new(
        1607392319,
        "Simple Model",
        vec![
            Field::new("Word"),
            Field::new("Wordh"),
            Field::new("Phrase"),
        ],
        vec![Template::new("Card 1")
            .qfmt("{{Word}}{{Wordh}}")
            .afmt(r#"{{FrontSide}}<hr id="answer"><br/>{{Wordh}}"#)],
    );
    let mut my_deck = Deck::new(
        2059400110,
        "3000-frequently-used-words",
        "Deck for studying country capitals",
    );
    for c in cards {
        let p = c.phrases.as_ref();
        let word_href = format!("<a href=\"{}{}\">{}{}</a>", DICT, c.item, DICT, c.item);
        let my_note = Note::new(
            my_model.clone(),
            vec![c.item.as_ref(), word_href.as_str(), p],
        )
        .unwrap();
        my_deck.add_note(my_note);
    }
    my_deck.write_to_file("vocabulary.apkg").unwrap();
}
fn parse_phrase(html: &str) -> Result<Option<Vec<String>>> {
    let mut phrases = vec![];
    let (html, sel) = selector_parse_doc(html, PHRASE_BLOCK)?;
    let ret = if let Some(element) = html.select(&sel).next() {
        let html = element.html();
        let (html, sel) = selector_parse_frac(&html, "a")?;
        for e in html.select(&sel) {
            //   parse phrases
            let pr = e
                .text()
                .map(<str as ToString>::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            phrases.push(pr);
        }
        if phrases.is_empty() {
            None
        } else {
            Some(phrases)
        }
    } else {
        None
    };
    Ok(ret)
}

#[test]
fn test_parse_definition() {
    let s = fs::read_to_string("give.html").unwrap();

    println!("{:?}", parse_phrase(&s).unwrap());
}
