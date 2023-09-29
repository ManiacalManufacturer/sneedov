#![crate_name = "sneedov"]

use std::env;
use std::sync::Arc;

use sneedov::database::SqliteDB;
use sneedov::markov::sneedov_feed;
use sneedov::telegram::start_dispatcher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    {
        let args: Vec<String> = env::args().collect();
        if args.len() > 1 {
            use std::time::Instant;
            let now = Instant::now();

            let path_name = format!("./{d}/model.db", d = &args[2]);
            let path = std::path::Path::new(&path_name);
            let database = SqliteDB::new(path).await?;

            if let Err(e) = sneedov_feed(&args[1], Arc::new(database)).await {
                eprintln!("Could not feed and seed: {}", e);
                return Err(e);
            }

            let elapsed = now.elapsed();
            eprintln!("Time elapsed: {:.2?}\n", elapsed);
        }
    }
    start_dispatcher().await?;

    Ok(())
}
