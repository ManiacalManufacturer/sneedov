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

macro_rules! get_occurrence {
    ($db:ident, $e:expr) => {{
        let result;
        let vec = $db.get_single_occurrences($e).await?;
        {
            let mut rng = thread_rng();
            result = vec.choose_weighted(&mut rng, |item| item.1)?.0;
        }
        result
    }};
    ($db:ident, $e1:expr, $e2:expr) => {{
        let result: (u64, u64);
        let vec = $db.get_double_occurrences($e1, $e2).await?;
        {
            let mut rng = thread_rng();
            result = *vec.choose_weighted(&mut rng, |item| item.1)?;
        }
        result
    }};
    (reverse $db:ident, $e:expr) => {{
        let result;
        let vec = $db.get_prev_single_occurrences($e).await?;
        {
            let mut rng = thread_rng();
            result = vec.choose_weighted(&mut rng, |item| item.1)?.0;
        }
        result
    }};
    (reverse $db:ident, $e1:expr, $e2:expr) => {{
        let result: (u64, u64);
        let vec = $db.get_prev_double_occurrences($e1, $e2).await?;
        {
            let mut rng = thread_rng();
            result = *vec.choose_weighted(&mut rng, |item| item.1)?;
        }
        result
    }};
}

macro_rules! generate {
    ($self:ident, $index:expr, $old_index:expr, $end:expr) => {{
        let mut index = $index;
        let mut index_result;
        let mut old_index = $old_index;
        let mut sentence = String::new();

        loop {
            (old_index, index_result) = (index, $self.next_word(old_index, index).await);
            match index_result {
                Ok(i) => {
                    index = i;
                }
                Err(_) => {
                    break;
                }
            }

            if index == $end {
                break;
            }

            let word = $self.get_word(index).await?;
            let is_punc = is_punctuation(word.parse::<char>());

            if sentence.len() != 0 && !is_punc {
                sentence.push_str(" ");
            }
            sentence.push_str(&word);
        }

        sentence
    }};
    (reply $self:ident, $index:expr, $old_index:expr, $end:expr) => {{
        let mut index = $index;
        let mut index_result; //temp solution
        let mut old_index = $old_index;
        let mut sentence = String::new();

        loop {
            if index == $end {
                break;
            }

            let word = $self.get_word(index).await?;
            let is_punc = is_punctuation(word.parse::<char>());

            if sentence.len() != 0 && !is_punc {
                sentence.push_str(" ");
            }

            sentence.push_str(&word);
            (old_index, index_result) = (index, $self.next_word(old_index, index).await);
            match index_result {
                Ok(i) => {
                    index = i;
                }
                Err(_) => {
                    break;
                }
            }
        }

        sentence
    }};
    (reverse $self:ident, $index:expr, $old_index:expr, $end:expr) => {{
        let mut index = $index;
        let mut index_result; //temp solution
                              //
        let mut old_index = $old_index;
        let mut sentence = String::new();

        let mut was_punc = false;

        loop {
            if index == $end {
                break;
            }

            let word = $self.get_word(index).await?;
            let is_punc = is_punctuation(word.parse::<char>());

            if sentence.len() != 0 && !was_punc {
                sentence.insert_str(0, " ");
            } else {
                was_punc = false;
            }

            if is_punc {
                was_punc = true;
            }

            sentence.insert_str(0, &word);
            (old_index, index_result) = (index, $self.prev_word(index, old_index).await);
            match index_result {
                Ok(i) => {
                    index = i;
                }
                Err(_) => {
                    break;
                }
            }
        }

        sentence
    }};
}

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
        //let mut result;
        //let hybrid;
        let database = &self.database;

        match self.markov_type {
            MarkovType::Single => {
                //let vec = self.database.get_single_occurrences(index2).await?;
                //let mut rng = thread_rng();
                //result = vec.choose_weighted(&mut rng, |item| item.1)?.0;
                Ok(get_occurrence!(database, index2))
            }
            MarkovType::Double => {
                //let vec = self.database.get_double_occurrences(index1, index2).await?;
                //let mut rng = thread_rng();
                //result = vec.choose_weighted(&mut rng, |item| item.1)?.0;
                Ok(get_occurrence!(database, index1, index2).0)
            }
            MarkovType::Hybrid(t) => {
                //let vec = self.database.get_double_occurrences(index1, index2).await?;
                //{
                //    let mut rng = thread_rng();
                //    let tuple = vec.choose_weighted(&mut rng, |item| item.1)?;
                //    result = tuple.0;
                //    hybrid = tuple.1;
                //}
                let tuple = get_occurrence!(database, index1, index2);
                if tuple.1 < t {
                    //let vec = self.database.get_single_occurrences(index2).await?;
                    //let mut rng = thread_rng();
                    //result = vec.choose_weighted(&mut rng, |item| item.1)?.0;
                    Ok(get_occurrence!(database, index2))
                } else {
                    Ok(tuple.0)
                }
            }
        }

        //Ok(result)
    }

    async fn prev_word(&self, index1: u64, index2: u64) -> Result<u64, Error> {
        let database = &self.database;

        match self.markov_type {
            MarkovType::Single => Ok(get_occurrence!(reverse database, index1)),
            MarkovType::Double => Ok(get_occurrence!(reverse database, index1, index2).0),
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

    pub async fn generate_reply(&mut self, line: String) -> Result<String, Error> {
        let split = split_sentence(line);
        let database = &self.database;

        //STEP 1 Separate the words
        let word;
        {
            let mut rng = thread_rng();
            word = split.choose(&mut rng).unwrap();
        }

        //STEP 2 Select the word from the database
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

        //STEP 3 Match depending on the keyword
        if keyword == "first" {
            Ok(generate!(reply self, index, START_INDEX, END_INDEX))
        } else if keyword == "last" {
            Ok(generate!(reverse self, index, END_INDEX, START_INDEX))
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
                first_half.push_str(" ");
            }
            Ok(first_half + &second_half)
            //let err: Error = String::from("Placeholder").into();
            //Err(err)
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
