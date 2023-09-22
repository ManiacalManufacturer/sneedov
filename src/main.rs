#![crate_name = "sneedov"]

use std::env;

use sneedov::markov::{sneedov_feed, sneedov_generate};

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let flags = sqlite::OpenFlags::new()
        .set_create()
        .set_full_mutex()
        .set_read_write();
    let path_name = "./test.db";
    let path = std::path::Path::new(path_name);
    let connection = sqlite::Connection::open_with_flags(path, flags)?;

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        use std::time::Instant;
        let now = Instant::now();
        if let Err(e) = sneedov_feed(&args[1], &connection) {
            eprintln!("Could not feed and seed: {}", e);
            return Err(e);
        }
        let elapsed = now.elapsed();
        eprintln!("Time elapsed: {:.2?}\n", elapsed);
    }


    let generation = sneedov_generate(&connection);
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
