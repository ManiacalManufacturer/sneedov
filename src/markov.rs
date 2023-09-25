use super::database::Database;
use super::split::{is_punctuation, split_sentence};

use indicatif::ProgressIterator;
use itertools::{Itertools, Position};
use rand::prelude::*;

use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::BufReader;

const START_KEYWORD: (&str, &str) = ("start", "");
const END_KEYWORD: (&str, &str) = ("end", "");

const START_INDEX: u64 = 2;
const END_INDEX: u64 = 1;

const HYBRID_THRESHOLD: u64 = 10;

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Default)]
pub enum MarkovType {
    Single,
    Double,
    #[default]
    Hybrid,
}

pub struct Markov {
    database: Box<dyn Database>,
    markov_type: MarkovType,
    generation: String,
}

impl Markov {
    pub fn new(database: Box<dyn Database>) -> Result<Self, Error> {
        let markov = Markov {
            database,
            markov_type: MarkovType::Hybrid,
            generation: String::new(),
        };
        markov.database.add_word(END_KEYWORD)?;
        markov.database.add_word(START_KEYWORD)?;
        Ok(markov)
    }

    pub fn append_line(&self, line: String) -> Result<(), Error> {
        let split = split_sentence(line);
        let iter = split.iter();

        let (mut prev_key, mut prev_word) = START_KEYWORD;
        let (mut curr_key, mut curr_word) = START_KEYWORD;

        for word in iter.with_position() {
            match word {
                (Position::First, w) => {
                    self.append_word((prev_key, prev_word), (curr_key, curr_word), ("first", w))?;
                    (curr_key, curr_word) = ("first", w);
                }
                (Position::Middle, w) => {
                    self.append_word((prev_key, prev_word), (curr_key, curr_word), ("middle", w))?;
                    (prev_key, prev_word) = (curr_key, curr_word);
                    (curr_key, curr_word) = ("middle", w);
                }
                (Position::Last, w) => {
                    self.append_word((prev_key, prev_word), (curr_key, curr_word), ("last", w))?;
                    self.append_word((curr_key, curr_word), ("last", w), END_KEYWORD)?;
                }
                (Position::Only, w) => {
                    self.append_word(START_KEYWORD, START_KEYWORD, ("last", w))?;
                    self.append_word(START_KEYWORD, ("last", w), END_KEYWORD)?;
                }
            }
        }

        Ok(())
    }

    fn next_word(&self, index1: u64, index2: u64) -> Result<u64, Error> {
        let mut rng = thread_rng();

        match self.markov_type {
            MarkovType::Single => {
                let vec = self.database.get_single_occurrences(index2)?;
                Ok(vec.choose_weighted(&mut rng, |item| item.1)?.0)
            }
            MarkovType::Double => {
                let vec = self.database.get_double_occurrences(index1, index2)?;
                Ok(vec.choose_weighted(&mut rng, |item| item.1)?.0)
            }
            MarkovType::Hybrid => {
                let vec = self.database.get_double_occurrences(index1, index2)?;
                let result = vec.choose_weighted(&mut rng, |item| item.1)?;
                if result.1 < HYBRID_THRESHOLD {
                    let vec = self.database.get_single_occurrences(index2)?;
                    Ok(vec.choose_weighted(&mut rng, |item| item.1)?.0)
                } else {
                    Ok(result.0)
                }
            }
        }
    }

    pub fn generate(&mut self) -> Result<String, Error> {
        let mut index = START_INDEX;
        let mut old_index = index;

        loop {
            (old_index, index) = (index, self.next_word(old_index, index)?);

            if index == END_INDEX {
                break;
            }

            let word = self.database.get_word(index)?;
            let is_punc = is_punctuation(word.parse::<char>());

            if self.generation.len() != 0 && !is_punc {
                self.generation.push_str(" ");
            }

            self.generation.push_str(&word);
        }

        let sentence = &self.generation;
        Ok(sentence.to_string())
    }

    fn append_word(
        &self,
        prev: (&str, &str),
        curr: (&str, &str),
        next: (&str, &str),
    ) -> Result<(), Error> {
        let (index1, index2, index3) = (
            self.database.add_word(prev)?,
            self.database.add_word(curr)?,
            self.database.add_word(next)?,
        );

        self.database.increment(index1, index2, index3)
    }
}

pub fn sneedov_feed(
    old_filename: &str,
    connection: &sqlite::Connection,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let file = OpenOptions::new().read(true).open(old_filename)?;

    let mut reader = BufReader::new(&file);
    let mut string = String::new();

    set_keywords(&connection)?;

    let _ = reader.read_to_string(&mut string);
    let vec: Vec<&str> = string.split("\n").collect();
    let iter = vec.iter();
    for line in iter.progress() {
        //let words = split_sentence!(line);
        //count_adjacent(&words);
        if line != &"" {
            sneedov_append_line(&connection, line)?;
        }
    }

    Ok(())
}
