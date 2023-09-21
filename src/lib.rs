mod database;
use database::{database, DatabaseMessage, DatabaseResult};

use indicatif::ProgressIterator;
use itertools::{Itertools, Position};
use rand::prelude::*;

use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::BufReader;

const START_KEYWORD: &str = "start";
const END_KEYWORD: &str = "end";
const PUNCTUATION: [char; 6] = ['.', ',', '?', '!', ';', ':'];

fn split_punctuation(split: Vec<&str>) -> Vec<String> {
    let list = split.iter();
    let mut vec = Vec::<String>::new();
    for value in list {
        let mut word = value.chars();
        let last_char = word.next_back();
        match last_char {
            Some(..) => {
                let mut clone = word.clone();
                let second_last_char = clone.next_back();
                match second_last_char {
                    Some(..) => {
                        if last_char.unwrap() != second_last_char.unwrap()
                            && PUNCTUATION.contains(&last_char.unwrap())
                            && second_last_char.unwrap() != '!'
                            && second_last_char.unwrap() != '?'
                        {
                            vec.push(String::from(word.as_str()));
                            vec.push(String::from(last_char.unwrap()));
                        } else {
                            vec.push(String::from(*value))
                        }
                    }
                    None => vec.push(String::from(*value)),
                }
            }
            None => (),
        }
    }
    vec
}

fn split_sentence(line: String) -> Vec<String> {
    let vec: Vec<&str> = line.split_whitespace().collect();
    let new_vec = split_punctuation(vec);
    new_vec
}

fn increment_next_word(
    filename: String,
    index1: usize,
    index2: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    database(filename, DatabaseMessage::Increment(index1, index2))?;
    Ok(())
}

fn get_occurrences(
    filename: String,
    index: usize,
) -> Result<Vec<(usize, usize)>, Box<dyn std::error::Error>> {
    if let DatabaseResult::VecTuple(vec) = database(filename, DatabaseMessage::GetNextWords(index))?
    {
        return Ok(vec);
    }
    let err: Box<dyn std::error::Error> = "Could not get vec!".into();
    Err(err)
}

fn get_word(filename: String, index: usize) -> Result<String, Box<dyn std::error::Error>> {
    // let mut split_line = line.split_whitespace();
    // //split_line.next().unwrap().to_owned()
    // if let Some(word) = split_line.next() {
    //     return Ok(word.to_owned());
    // }
    if let DatabaseResult::String(string) = database(filename, DatabaseMessage::GetWord(index))? {
        return Ok(string);
    }
    let err: Box<dyn std::error::Error> =
        String::from("None was returned. Is your file corrupted or missing?").into();

    Err(err)
}

fn get_next_word(vec: Vec<(usize, usize)>) -> Result<usize, Box<dyn std::error::Error>> {
    let mut rng = thread_rng();
    Ok(vec.choose_weighted(&mut rng, |item| item.1)?.0)
}

fn add_word(
    filename: &str,
    keyword: &str,
    string: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    if let DatabaseResult::Int(id) = database(
        filename.to_owned(),
        DatabaseMessage::AddWord(keyword.to_owned(), string.to_owned()),
    )? {
        return Ok(id);
    }

    let err: Box<dyn std::error::Error> = "Could not return id".into();
    Err(err)
}

fn is_punctuation(charresult: Result<char, std::char::ParseCharError>) -> bool {
    match charresult {
        Ok(c) => PUNCTUATION.contains(&c),
        Err(_) => false,
    }
}

fn set_keywords(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e1) = add_word(filename, END_KEYWORD, "") {
        return Err(e1);
    }
    if let Err(e2) = add_word(filename, START_KEYWORD, "") {
        return Err(e2);
    }
    Ok(())
}

///Appends a word and increments the occurance of the next word to a file
///
///# Arguments
///
///* `filename` -  File name for the file to save to
///* `word` -  The word that gets appended
///* `next_word` -  The future word that occurs after the current one
///
pub fn sneedov_append_word(
    filename: &str,
    keyword: &str,
    string: &str,
    next_keyword: &str,
    next_string: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let index1 = add_word(filename, keyword, string)?;
    let index2 = add_word(filename, next_keyword, next_string)?;

    increment_next_word(filename.into(), index1, index2)?;
    Ok(())
}

pub fn sneedov_append_line(filename: &str, line: &str) -> Result<(), Box<dyn std::error::Error>> {
    let split = split_sentence(line.to_owned());
    let iter = split.iter();

    let mut previous = String::new();
    let mut previous_keyword = "first";
    for word in iter.with_position() {
        match word {
            (Position::First, w) => {
                sneedov_append_word(filename, START_KEYWORD, "", previous_keyword, w.as_str())?;
                previous = w.to_owned();
            }
            (Position::Middle, w) => {
                sneedov_append_word(
                    filename,
                    previous_keyword,
                    previous.as_str(),
                    "middle",
                    w.as_str(),
                )?;
                previous = w.to_owned();
                previous_keyword = "middle";
            }
            (Position::Last, w) => {
                sneedov_append_word(
                    filename,
                    previous_keyword,
                    previous.as_str(),
                    "last",
                    w.as_str(),
                )?;
                sneedov_append_word(filename, "last", w.as_str(), END_KEYWORD, "")?;
            }
            (Position::Only, w) => {
                sneedov_append_word(filename, START_KEYWORD, "", "last", w.as_str())?;
                sneedov_append_word(filename, "last", w.as_str(), END_KEYWORD, "")?;
            }
        }
    }

    Ok(())
}

///Generates a new sentence from a markov chain file
///
///# Arguments
///
///* `filename` -  File name for the file to load from
///
pub fn sneedov_generate(filename: &str) -> Result<String, Box<dyn std::error::Error>> {
    //code goes here
    let mut index = 2;
    let mut sentence = String::new();

    loop {
        index = get_next_word(get_occurrences(filename.to_owned(), index)?)?;

        if index == 1 {
            break;
        }

        let word = get_word(filename.to_owned(), index)?;

        let is_punc: bool = is_punctuation(word.parse::<char>());

        if sentence.len() != 0 && !is_punc {
            sentence.push_str(" ");
        }
        sentence.push_str(&word);
    }

    Ok(sentence)
}

pub fn sneedov_feed(filename: &str, new_filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = OpenOptions::new().read(true).open(filename)?;

    let mut reader = BufReader::new(&file);
    let mut string = String::new();

    set_keywords(new_filename)?;

    let _ = reader.read_to_string(&mut string);
    let vec: Vec<&str> = string.split("\n").collect();
    let iter = vec.iter();
    for line in iter.progress() {
        //let words = split_sentence!(line);
        //count_adjacent(&words);
        if line != &"" {
            if let Err(e) = sneedov_append_line(new_filename, line) {
                eprintln!("Error feeding: {}", e);
            }
        }
    }

    Ok(())
}
