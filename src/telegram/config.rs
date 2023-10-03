use super::super::markov::MarkovType;
use serde::{Deserialize, Serialize};
use tokio::fs::{create_dir_all, read_to_string, File};
use tokio::io::AsyncWriteExt;
use toml;

#[derive(Serialize, Deserialize)]
pub struct Secret {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct Reply {
    pub enabled: bool,
    pub unique: bool,
}

#[derive(Serialize, Deserialize)]
pub struct MarkovConfig {
    pub chance: Option<u64>,
    pub markov_type: MarkovType,
    pub reply: Reply,
}

type Error = Box<dyn std::error::Error + Send + Sync>;

pub async fn get_secret() -> Result<Secret, Error> {
    let path_name = format!("secret.toml");
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
            write_default(dir, path, toml).await?;
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
            let config = MarkovConfig {
                chance: Some(10),
                markov_type: MarkovType::default(),
                reply: Reply {
                    enabled: true,
                    unique: false,
                },
            };

            let toml = toml::to_string(&config)?;
            write_default(dir, path, toml).await?;
            string = read_to_string(path).await?;
        }
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    let mut config: MarkovConfig = toml::from_str(&string)?;
    if config.chance.is_none() {
        config.chance = Some(10);
    }
    Ok(config)
}

async fn write_default(
    dir: &std::path::Path,
    path: &std::path::Path,
    string: String,
) -> Result<(), Error> {
    create_dir_all(dir).await?;

    let mut file = File::create(path).await?;
    file.write_all(string.as_bytes()).await?;
    Ok(())
}
