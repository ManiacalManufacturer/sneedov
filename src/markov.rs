use super::database::Database;

use rand::prelude::*;
use serde::{Deserialize, Serialize};

use futures::future::try_join_all;
use futures::stream::{futures_unordered::FuturesUnordered, StreamExt};
use std::fs::OpenOptions;
use std::io::{prelude::*, BufReader};
use std::sync::Arc;

pub mod split;
use split::{is_punctuation, split_sentence};

const START_KEYWORD: (&str, &str) = ("start", "");
const END_KEYWORD: (&str, &str) = ("end", "");

const START_INDEX: u64 = 2;
const END_INDEX: u64 = 1;

const DEFAULT_HYBRID_THRESHOLD: u64 = 10;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "hybrid-threshold")]
pub enum MarkovType {
    Single,
    Double,
    Hybrid(u64),
}

impl Default for MarkovType {
    fn default() -> Self {
        MarkovType::Hybrid(DEFAULT_HYBRID_THRESHOLD.into())
    }
}

type DatabaseType = Arc<dyn Database + Send + Sync>;

pub struct Markov {
    database: DatabaseType,
    markov_type: MarkovType,
}

pub struct MarkovBuilder {
    database: DatabaseType,
    markov_type: MarkovType,
}

impl MarkovBuilder {
    pub fn new(database: DatabaseType) -> MarkovBuilder {
        MarkovBuilder {
            database,
            markov_type: MarkovType::default(),
        }
    }

    pub fn markov_type(mut self, markov_type: MarkovType) -> MarkovBuilder {
        self.markov_type = markov_type;
        self
    }

    pub async fn build(self) -> Result<Markov, Error> {
        self.database.add_word(END_KEYWORD).await?;
        self.database.add_word(START_KEYWORD).await?;
        Ok(Markov {
            database: self.database,
            markov_type: self.markov_type,
        })
    }
}

impl Markov {
    pub async fn new(database: DatabaseType) -> Result<Self, Error> {
        let markov = Markov {
            database,
            markov_type: MarkovType::default(),
        };

        markov.database.add_word(END_KEYWORD).await?;
        markov.database.add_word(START_KEYWORD).await?;
        Ok(markov)
    }

    pub fn builder(database: DatabaseType) -> MarkovBuilder {
        MarkovBuilder::new(database)
    }

    pub async fn append_line(&self, line: String) -> Result<(), Error> {
        let split = split_sentence(line);

        let (mut prev, mut curr) = (START_KEYWORD, START_KEYWORD);
        let mut next = START_KEYWORD;

        let length = split.len();
        let mut index = 0;

        let mut futures = split
            .iter()
            .map(|word| {
                index += 1;
                prev = curr;
                curr = next;
                if index == length {
                    next = ("last", word);
                } else if index == 1 {
                    next = ("first", word);
                } else {
                    next = ("middle", word);
                }
                self.append_word(prev, curr, next)
            })
            .collect::<FuturesUnordered<_>>();

        futures.push(self.append_word(curr, next, END_KEYWORD));
        while let Some(res) = futures.next().await {
            res?;
        }
        Ok(())
    }

    async fn next_word(&self, index1: u64, index2: u64) -> Result<u64, Error> {
        let mut result;

        match self.markov_type {
            MarkovType::Single => {
                let vec = self.database.get_single_occurrences(index2).await?;
                let mut rng = thread_rng();
                result = vec.choose_weighted(&mut rng, |item| item.1)?.0;
            }
            MarkovType::Double => {
                let vec = self.database.get_double_occurrences(index1, index2).await?;
                let mut rng = thread_rng();
                result = vec.choose_weighted(&mut rng, |item| item.1)?.0;
            }
            MarkovType::Hybrid(t) => {
                let vec = self.database.get_double_occurrences(index1, index2).await?;
                {
                    let mut rng = thread_rng();
                    result = vec.choose_weighted(&mut rng, |item| item.1)?.0;
                }
                if result < t {
                    let vec = self.database.get_single_occurrences(index2).await?;
                    let mut rng = thread_rng();
                    result = vec.choose_weighted(&mut rng, |item| item.1)?.0;
                }
            }
        }

        Ok(result)
    }
    async fn get_word(&self, index: u64) -> Result<String, Error> {
        Ok(self.database.get_word(index).await?)
    }

    pub async fn generate(&mut self) -> Result<String, Error> {
        let mut index = START_INDEX;
        let mut old_index = index;
        let mut sentence = String::new();

        loop {
            (old_index, index) = (index, self.next_word(old_index, index).await?);

            if index == END_INDEX {
                break;
            }

            let word = self.get_word(index).await?;
            let is_punc = is_punctuation(word.parse::<char>());

            if sentence.len() != 0 && !is_punc {
                sentence.push_str(" ");
            }

            sentence.push_str(&word);
        }

        Ok(sentence)
    }

    async fn append_word(
        &self,
        prev: (&str, &str),
        curr: (&str, &str),
        next: (&str, &str),
    ) -> Result<(), Error> {
        let vec = try_join_all(vec![
            self.database.add_word(prev),
            self.database.add_word(curr),
            self.database.add_word(next),
        ])
        .await?;
        self.database.increment(vec[0], vec[1], vec[2]).await?;
        Ok(())
    }
}

pub async fn sneedov_feed(filename: &str, database: DatabaseType) -> Result<(), Error> {
    let file = OpenOptions::new().read(true).open(filename)?;

    let mut reader = BufReader::new(&file);
    let mut string = String::new();

    let _ = reader.read_to_string(&mut string);
    let vec: Vec<&str> = string.split("\n").collect();

    let markov = Markov::new(database).await?;

    let length = vec.len() as u64;
    let bar = indicatif::ProgressBar::new(length);

    let mut futures = vec
        .iter()
        .filter(|x| if *x != &"" { true } else { false })
        .map(|x| markov.append_line(x.to_string()));

    while let Some(res) = futures.next() {
        res.await?;
        bar.inc(1);
    }

    Ok(())
}
