const PUNCTUATION: [char; 6] = ['.', ',', '?', '!', ';', ':'];

fn split_punctuation(split: Vec<&str>) -> Vec<String> {
    //TODO: Make this less ugly
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

pub fn split_sentence(line: String) -> Vec<String> {
    let vec: Vec<&str> = line.split_whitespace().collect();
    let new_vec = split_punctuation(vec);
    new_vec
}

pub fn is_punctuation(charresult: Result<char, std::char::ParseCharError>) -> bool {
    match charresult {
        Ok(c) => PUNCTUATION.contains(&c),
        Err(_) => false,
    }
}
