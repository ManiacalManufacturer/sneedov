const PUNCTUATION: [char; 6] = ['.', ',', '?', '!', ';', ':'];

fn split_punctuation(split: &[&str]) -> Vec<String> {
    //TODO: Make this less ugly
    let list = split.iter();
    let mut vec = Vec::<String>::new();
    for value in list {
        let mut word = value.chars();

        if let Some(last_char) = word.next_back() {
            let mut clone = word.clone();
            if let Some(second_last_char) = clone.next_back() {
                if last_char != second_last_char
                    && PUNCTUATION.contains(&last_char)
                    && second_last_char != '!'
                    && second_last_char != '?'
                {
                    vec.push(word.as_str().into());
                    vec.push(last_char.to_string());
                } else {
                    vec.push(value.to_string());
                }
            } else {
                vec.push(value.to_string());
            }
        }
    }
    vec
}

pub fn split_sentence(line: &str) -> Vec<String> {
    let vec: Vec<&str> = line.split_whitespace().collect();
    split_punctuation(&vec)
}

pub fn is_punctuation(charresult: Result<char, std::char::ParseCharError>) -> bool {
    match charresult {
        Ok(c) => PUNCTUATION.contains(&c),
        Err(_) => false,
    }
}
