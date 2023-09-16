use sneedov::set_keywords;
use sneedov::sneedov_append;

fn split_punctuation(split: Vec<&str>) -> Vec<String> {
    let punctuation = ['.', ',', '?', '!', ';', ':'];

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
                            && punctuation.contains(&last_char.unwrap())
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

//testtesti

//anothertest

macro_rules! split_sentence {
    ($x:expr) => {{
        let vec: Vec<&str> = $x.split_whitespace().collect();
        let new_vec = split_punctuation(vec);
        new_vec
    }};
}

fn count_adjacent(vec: &Vec<String>) {
    let mut iter = vec.iter().peekable();

    if iter.peek().is_none() {
        println!("Line is empty");
    }

    let mut is_first: bool = true;
    while let Some(value) = iter.next() {
        if is_first {
            let _ = sneedov_append("test", "__start", value);
            is_first = false;
        }
        //print!("{} {} ", previous, value);
        if iter.peek().is_none() {
            //println!("{}", "__end");
            let _ = sneedov_append("test", value, "__end");
        } else {
            //println!("{}", iter.peek().unwrap());
            let _ = sneedov_append("test", value, iter.peek().unwrap());
        }
    }
}

fn main() {
    let sentence: &str = "🤣 What is BRUHHH even doing in a ohio 💀 town!";

    let _ = set_keywords("test");
    let words = split_sentence!(sentence);
    count_adjacent(&words);
}
