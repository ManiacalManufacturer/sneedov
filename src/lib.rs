use rand::prelude::*;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::BufReader;

const START_KEYWORD: &str = "__start";
const END_KEYWORD: &str = "__end";
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

macro_rules! split_sentence {
    ($x:expr) => {{
        let vec: Vec<&str> = $x.split_whitespace().collect();
        let new_vec = split_punctuation(vec);
        new_vec
    }};
}

fn return_index(file: &File, word: &str) -> Option<usize> {
    let reader = BufReader::new(file);
    let lines = reader.lines();

    for (index, instance) in lines.enumerate() {
        let unwrapped = instance.unwrap();
        let entry = split_sentence!(unwrapped);
        if let Some(thing) = entry.first() {
            if thing == word {
                return Some(index);
            }
        }
    }
    None
}

fn write_to_file(file: &mut File, string: &str) -> Result<(), std::io::Error> {
    if let Err(e) = writeln!(file, "{}", string) {
        eprintln!("Error: Could not write to file: {}", e);
        return Err(e);
    }
    Ok(())
}

fn increment_line(line: &str, index: usize) -> String {
    let split_line = line.split_whitespace();
    let mut new_line: String = String::new();

    let mut _current_word: String = String::new();
    let mut exists: bool = false;

    for word in split_line {
        _current_word = word.to_string();
        let mut split = word.split(":");
        if let Some(i) = split.next() {
            if i == index.to_string() {
                let count: usize = split.next().unwrap().parse::<usize>().unwrap() + 1;
                _current_word = format!("{}:{}", index, count);
                exists = true;
            }
        }
        new_line.push_str(&_current_word);
        new_line.push_str(" ");
    }

    if !exists {
        _current_word = format!("{}:{}", index, 1);
        new_line.push_str(&_current_word);
    }
    new_line
}

fn get_occurances(line: &str) -> Result<Vec<(usize, usize)>, Box<dyn std::error::Error>> {
    let mut split_line = line.split_whitespace();
    let mut vec: Vec<(usize, usize)> = vec![];

    split_line.next();
    for word in split_line {
        let mut split = word.split(":");
        let tuple = (
            split.next().unwrap().parse::<usize>().unwrap(),
            split.next().unwrap().parse::<usize>().unwrap(),
        );
        vec.push(tuple)
    }
    Ok(vec)
}

fn get_line(filename: &str, index: usize) -> Result<String, std::io::Error> {
    let file = OpenOptions::new()
        .read(true)
        .open(format!("{}words", filename))?;

    let mut reader = BufReader::new(&file);
    let mut string = String::new();
    let _ = reader.read_to_string(&mut string);

    let line = string.split("\n").nth(index).unwrap().to_owned();

    Ok(line)
}

fn get_word(line: &String) -> String {
    let mut split_line = line.split_whitespace();
    split_line.next().unwrap().to_owned()
}

fn get_next_word(vec: Vec<(usize, usize)>) -> usize {
    //let mut total_vec: Vec<usize> = vec![];
    //let tuple_vec: Vec<(usize, usize)> = vec.clone();

    // for instance in vec {
    //     total_vec.push(instance.1);
    // }

    // let sum: usize = total_vec.iter().sum();
    // let mut adjusted_vec: Vec<(usize, usize)> = tuple_vec.clone();

    // for instance in tuple_vec {
    //     let divided = instance.1 / sum;
    //     adjusted_vec.push((instance.0, divided));
    // }

    let mut rng = thread_rng();
    vec.choose_weighted(&mut rng, |item| item.1).unwrap().0
}

fn add_word(filename: &str, keyword: &str) -> Result<usize, std::io::Error> {
    let mut file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .open(format!("{}words", filename))
        .unwrap();

    if let Some(index) = return_index(&file, keyword) {
        return Ok(index);
    } else {
        if let Err(e) = write_to_file(&mut file, keyword) {
            return Err(e);
        }
    }
    let _ = file.flush();
    file.rewind().unwrap();
    let mut count: isize = -1;
    let lines = BufReader::new(&file).lines();
    for _ in lines {
        count += 1;
    }
    Ok(count as usize)
}

fn add_sanitized_word(filename: &str, keyword: &str) -> Result<usize, std::io::Error> {
    if keyword == START_KEYWORD || keyword == END_KEYWORD {
        let mut new_word = String::new();
        new_word.push('_');
        new_word.push_str(keyword);
        return add_sanitized_word(filename, new_word.as_str());
    }
    add_word(filename, keyword)
}

fn fix_fake_keyword(keyword: &str) -> &str {
    if keyword == "___start" || keyword == "___end" {
        let mut chars = keyword.chars();
        chars.next();
        return chars.as_str();
    }
    keyword
}

fn is_punctuation(charresult: Result<char, std::char::ParseCharError>) -> bool {
    match charresult {
        Ok(c) => PUNCTUATION.contains(&c),
        Err(_) => false,
    }
}

pub fn set_keywords(filename: &str) -> Result<(), std::io::Error> {
    if let Err(e1) = add_word(filename, END_KEYWORD) {
        return Err(e1);
    }
    if let Err(e2) = add_word(filename, START_KEYWORD) {
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
    word: &str,
    next_word: &str,
) -> Result<(), std::io::Error> {
    let result = add_sanitized_word(filename, word);

    match result {
        Ok(i) => {
            let num = add_sanitized_word(filename, next_word);
            if let Err(e) = num {
                return Err(e);
            }

            let mut read_file = OpenOptions::new()
                .read(true)
                .append(false)
                .open(format!("{}words", filename))
                .unwrap();

            let reader = BufReader::new(&read_file);
            let mut lines = reader.lines();

            let specific_line = lines.nth(i).unwrap().unwrap();
            let new_line = increment_line(&specific_line, num.unwrap());
            read_file.rewind().unwrap();

            let mut write_file = OpenOptions::new()
                .read(true)
                .write(true)
                .append(false)
                .open(format!("{}words", filename))
                .unwrap();

            let mut reader = BufReader::new(&write_file);
            let mut string: String = String::new();
            let mut new_string: String = String::new();
            let _ = reader.read_to_string(&mut string);
            let mut vec: Vec<&str> = string.split("\n").collect();

            vec[i] = &new_line;
            for line in vec {
                if line != "" {
                    new_string.push_str(line);
                    new_string.push_str("\n");
                }
            }

            write_file.rewind().unwrap();
            if let Err(e) = write!(&mut write_file, "{}", new_string) {
                return Err(e);
            }
            let _ = write_file.flush();

            Ok(())
        }
        Err(e) => Err(e),
    }
}

///Generates a new sentence from a markov chain file
///
///# Arguments
///
///* `filename` -  File name for the file to load from
///
pub fn sneedov_generate(filename: &str) -> Result<String, Box<dyn std::error::Error>> {
    //code goes here
    let mut index = 1;
    let mut sentence = String::new();

    loop {
        let line = get_line(filename, index)?;
        let word = get_word(&line);

        if word == END_KEYWORD {
            break;
        }
        index = get_next_word(get_occurances(line.as_str()).unwrap());

        if word != START_KEYWORD {
            let is_punc: bool = is_punctuation(word.parse::<char>());

            if sentence.len() != 0 && !is_punc {
                sentence.push_str(" ");
            }
            sentence.push_str(fix_fake_keyword(&word));
        }
    }

    Ok(sentence)
}

pub fn sneedov_feed(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = OpenOptions::new().read(true).open("filename")?;

    let mut reader = BufReader::new(&file);
    let mut string = String::new();

    set_keywords("test")?;

    let _ = reader.read_to_string(&mut string);
    for line in string.split("\n") {
        let words = split_sentence!(line);
        //count_adjacent(&words);
    }

    Ok(())
}
