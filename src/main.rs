#![crate_name = "sneedov"]

// use sneedov::sneedov_append_word;
use sneedov::sneedov_generate;

fn main() {
    // let sentence: &str = "🤣 LMAOOOOOO im dead";

    // let _ = set_keywords("test");
    // let words = split_sentence!(sentence);
    // count_adjacent(&words);
    println!("{}", sneedov_generate("test").unwrap());
}
