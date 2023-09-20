#![crate_name = "sneedov"]

use std::env;

// use sneedov::sneedov_append_word;
use sneedov::{sneedov_feed, sneedov_generate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        if let Err(e) = sneedov_feed(&args[1], &args[2]) {
            eprintln!("Could not feed and seed: {}", e);
            return Err(e);
        }
    } else if args.len() > 1 {
        if let Err(e) = sneedov_feed(&args[1], "test") {
            eprintln!("Could not feed and seed: {}", e);
        }
    }

    // // let sentence: &str = "He will always be a gem 💎";
    // // if let Err(e) = sneedov_append_line("test", sentence) {
    // //     eprintln!("Error appending: {}", e);
    // // }
    // // let _ = set_keywords("test");
    // // let words = split_sentence!(sentence);
    // // count_adjacent(&words);

    let generation = sneedov_generate("test");
    match generation {
        Ok(gen) => {
            println!("{}", gen);
            Ok(())
        }
        Err(err) => {
            eprintln!("Could not generate: {}", err);
            Err(err)
        }
    }

    // let message = DatabaseMessage::AddWord("start".to_owned(), "".to_owned());
    // if let Err(e) = database("a".to_owned(), message).await {
    //     eprintln!("{}", e);
    // }
    //Ok(())
}
