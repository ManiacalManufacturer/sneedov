use super::super::markov::{MarkovType, ReplyMode};
use serde::{Deserialize, Serialize};
use tokio::fs::{create_dir_all, read_to_string, File};
use tokio::io::AsyncWriteExt;
use toml;

#[derive(Serialize, Deserialize)]
pub struct Secret {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct MarkovConfigToml {
    pub markov_type: Option<MarkovType>,
    pub chance: Option<u64>,
    pub reply_mode: Option<ReplyMode>,
}

pub struct MarkovConfig {
    pub markov_type: MarkovType,
    pub chance: u64,
    pub reply_mode: ReplyMode,
}

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn get_secret() -> Result<Secret, Error> {
    let path_name = "secret.toml";
    let path = std::path::Path::new(&path_name);
    let dir = std::path::Path::new("./");

    let result = read_to_string(path).await;
    let string;
    match result {
        Ok(s) => {
            string = s;
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            let config = Secret { token: "".into() };

            let toml = toml::to_string(&config)?;
            write_default(dir, path, &toml).await?;
            string = read_to_string(path).await?;
        }
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    Ok(toml::from_str(&string)?)
}

pub async fn get_config(filename: &str) -> Result<MarkovConfig, Error> {
    let path_name = format!("./{}/config.toml", filename);
    let dir_name = format!("./{}/", filename);
    let path = std::path::Path::new(&path_name);
    let dir = std::path::Path::new(&dir_name);

    let result = read_to_string(path).await;
    let string;
    match result {
        Ok(s) => {
            string = s;
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            let config = MarkovConfigToml {
                markov_type: Some(MarkovType::default()),
                chance: Some(10),
                reply_mode: Some(ReplyMode::default()),
            };

            let toml = toml::to_string(&config)?;
            write_default(dir, path, &toml).await?;
            string = read_to_string(path).await?;
        }
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    let config: MarkovConfigToml = toml::from_str(&string)?;

    let mut chance = 10;
    let mut markov_type = MarkovType::default();
    let mut reply_mode = ReplyMode::default();

    if let Some(value) = config.chance {
        chance = value;
    }
    if let Some(value) = config.markov_type {
        markov_type = value;
    }
    if let Some(value) = config.reply_mode {
        reply_mode = value;
    }
    Ok(MarkovConfig {
        chance,
        markov_type,
        reply_mode,
    })
}

async fn write_default(
    dir: &std::path::Path,
    path: &std::path::Path,
    string: &str,
) -> Result<(), Error> {
    create_dir_all(dir).await?;

    let mut file = File::create(path).await?;
    file.write_all(string.as_bytes()).await?;
    Ok(())
}
