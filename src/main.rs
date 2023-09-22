#![crate_name = "sneedov"]

use std::env;

use sneedov::{sneedov_feed, sneedov_generate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;
    let now = Instant::now();

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

    let elapsed = now.elapsed();
    eprintln!("Time elapsed: {:.2?}", elapsed);

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
}
