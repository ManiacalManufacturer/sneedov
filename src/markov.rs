use super::database::Database;

use rand::prelude::*;
use serde::{Deserialize, Serialize};

use futures::future::try_join_all;
use futures::stream::{futures_unordered::FuturesUnordered, StreamExt};
use std::fs::OpenOptions;
use std::io::{prelude::*, BufReader};
use std::sync::Arc;

pub mod macros;
pub mod split;
use macros::{generate, get_occurrence};
use split::{is_punctuation, split_sentence};

const START_KEYWORD: (&str, &str) = ("start", "");
const END_KEYWORD: (&str, &str) = ("end", "");

const START_INDEX: u64 = 2;
const END_INDEX: u64 = 1;

const DEFAULT_HYBRID_THRESHOLD: u64 = 10;
pub const DEFAULT_MARKOV_TYPE: MarkovType = MarkovType::Hybrid(DEFAULT_HYBRID_THRESHOLD);
pub const DEFAULT_REPLY_MODE: ReplyMode = ReplyMode::Reply;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(tag = "type", content = "hybrid-threshold")]
pub enum MarkovType {
    Single(u64),
    Double(u64),
    Hybrid(u64),
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum ReplyMode {
    Off,
    Random,
    Reply,
    ReplyUnique,
}

impl Default for MarkovType {
    fn default() -> Self {
        DEFAULT_MARKOV_TYPE
    }
}

impl Default for ReplyMode {
    fn default() -> Self {
        DEFAULT_REPLY_MODE
    }
}

type DatabaseType = Arc<dyn Database + Send + Sync>;

pub struct Markov {
    database: DatabaseType,
    markov_type: MarkovType,
    markov_chance: u64,
    reply_mode: ReplyMode,
}

pub struct MarkovBuilder {
    database: DatabaseType,
    markov_type: MarkovType,
    markov_chance: u64,
    reply_mode: ReplyMode,
}

impl MarkovBuilder {
    pub fn new(database: DatabaseType) -> MarkovBuilder {
        MarkovBuilder {
            database,
            markov_type: MarkovType::default(),
            markov_chance: 10,
            reply_mode: ReplyMode::default(),
        }
    }

    pub fn markov_type(mut self, markov_type: MarkovType) -> MarkovBuilder {
        self.markov_type = markov_type;
        self
    }

    pub fn markov_chance(mut self, markov_chance: u64) -> MarkovBuilder {
        self.markov_chance = markov_chance;
        self
    }

    pub fn reply_mode(mut self, reply_mode: ReplyMode) -> MarkovBuilder {
        self.reply_mode = reply_mode;
        self
    }

    pub async fn build(self) -> Result<Markov, Error> {
        self.database.add_word(END_KEYWORD).await?;
        self.database.add_word(START_KEYWORD).await?;
        Ok(Markov {
            database: self.database,
            markov_type: self.markov_type,
            markov_chance: self.markov_chance,
            reply_mode: self.reply_mode,
        })
    }
}

impl Markov {
    pub async fn new(database: DatabaseType) -> Result<Self, Error> {
        let markov = Markov {
            database,
            markov_type: MarkovType::default(),
            markov_chance: 10,
            reply_mode: ReplyMode::default(),
        };

        markov.database.add_word(END_KEYWORD).await?;
        markov.database.add_word(START_KEYWORD).await?;
        Ok(markov)
    }

    pub fn builder(database: DatabaseType) -> MarkovBuilder {
        MarkovBuilder::new(database)
    }

    pub fn chance(&self) -> bool {
        if self.markov_chance == 0 {
            return false;
        }

        let mut rng = thread_rng();
        if rng.gen_range(1..=self.markov_chance) == 1 {
            return true;
        }
        false
    }

    pub async fn append_line(&self, line: &str) -> Result<(), Error> {
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

        if next == START_KEYWORD {
            //This will never occur with teloxide
            panic!("Empty line");
        }

        futures.push(self.append_word(curr, next, END_KEYWORD));
        while let Some(res) = futures.next().await {
            res?;
        }
        Ok(())
    }

    async fn next_word(&self, index1: u64, index2: u64) -> Result<u64, Error> {
        let database = &self.database;

        match self.markov_type {
            MarkovType::Single(_) => Ok(get_occurrence!(database, index2)),
            MarkovType::Double(_) => Ok(get_occurrence!(database, index1, index2).0),
            MarkovType::Hybrid(t) => {
                let tuple = get_occurrence!(database, index1, index2);
                if tuple.1 < t {
                    Ok(get_occurrence!(database, index2))
                } else {
                    Ok(tuple.0)
                }
            }
        }
    }

    async fn prev_word(&self, index1: u64, index2: u64) -> Result<u64, Error> {
        let database = &self.database;

        match self.markov_type {
            MarkovType::Single(_) => Ok(get_occurrence!(reverse database, index1)),
            MarkovType::Double(_) => Ok(get_occurrence!(reverse database, index1, index2).0),
            MarkovType::Hybrid(t) => {
                let tuple = get_occurrence!(reverse database, index1, index2);
                if tuple.1 < t {
                    Ok(get_occurrence!(reverse database, index1))
                } else {
                    Ok(tuple.0)
                }
            }
        }
    }

    async fn get_word(&self, index: u64) -> Result<String, Error> {
        self.database.get_word(index).await
    }

    pub async fn generate(&self) -> Result<String, Error> {
        Ok(generate!(self, START_INDEX, START_INDEX, END_INDEX))
    }

    pub async fn generate_reply(&self, line: &str) -> Result<String, Error> {
        match &self.reply_mode {
            ReplyMode::Off => {
                return Ok("".to_owned());
            }
            ReplyMode::Random => {
                return Ok(generate!(self, START_INDEX, START_INDEX, END_INDEX));
            }
            _ => {}
        };

        let split = split_sentence(line);
        let database = &self.database;

        let word;
        {
            let mut rng = thread_rng();
            word = split.choose(&mut rng).unwrap();
        }

        let vec = self.database.get_case_insensitive(word).await?;
        let index;
        let keyword;
        {
            let mut rng = thread_rng();
            if let Some(tuple) = vec.choose(&mut rng) {
                index = tuple.0;
                keyword = tuple.1.to_owned();
            } else {
                let err: Error = String::from("Could not find similar words!").into();
                return Err(err);
            }
        }
        let sentence;

        if keyword == "first" {
            sentence = generate!(reply self, index, START_INDEX, END_INDEX);
        } else if keyword == "last" {
            sentence = generate!(reverse self, index, END_INDEX, START_INDEX);
        } else {
            let second = get_occurrence!(database, index);
            let mut first_half = generate!(reverse self, index, second, START_INDEX);
            let second_half = generate!(reply self, second, index, END_INDEX);

            let is_punc1 = {
                let x = first_half.clone().chars().next_back();
                match x {
                    Some(x) => is_punctuation(Ok(x)),
                    None => true,
                }
            };

            let is_punc2 = {
                let x = second_half.clone().chars().next();
                match x {
                    Some(x) => is_punctuation(Ok(x)),
                    None => true,
                }
            };

            if !is_punc1 && !is_punc2 {
                first_half.push(' ');
            }
            sentence = first_half + &second_half;
        }

        match &self.reply_mode {
            ReplyMode::ReplyUnique => {
                if line == sentence {
                    Ok(generate!(self, START_INDEX, START_INDEX, END_INDEX))
                } else {
                    Ok(sentence)
                }
            }
            _ => Ok(sentence),
        }
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
    let vec: Vec<&str> = string.split('\n').map(|x| x.trim()).collect();

    let markov = Markov::new(database).await?;

    let length = vec.len() as u64;
    let bar = indicatif::ProgressBar::new(length);

    let futures = vec
        .iter()
        .filter(|&x| !x.is_empty())
        .map(|x| markov.append_line(x));

    for res in futures {
        res.await?;
        bar.inc(1);
    }

    Ok(())
}
