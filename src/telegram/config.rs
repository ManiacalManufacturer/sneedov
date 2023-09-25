use super::super::markov::MarkovType;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, read_to_string, File};
use std::io::Write;
use toml;

#[derive(Serialize, Deserialize)]
pub struct Secret {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct MarkovConfig {
    pub markov_type: MarkovType,
    pub chance: Option<u64>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;

pub fn get_secret() -> Result<Secret, Error> {
    let path_name = format!("secret.toml");
    let path = std::path::Path::new(&path_name);
    let dir = std::path::Path::new("./");

    let result = read_to_string(path);
    let string;
    match result {
        Ok(s) => {
            string = s;
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            let config = Secret { token: "".into() };

            let toml = toml::to_string(&config)?;
            write_default(dir, path, toml)?;
            string = read_to_string(path)?;
        }
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    Ok(toml::from_str(&string)?)
}

pub fn get_config(filename: &str) -> Result<MarkovConfig, Error> {
    let path_name = format!("./{}/config.toml", filename);
    let dir_name = format!("./{}/", filename);
    let path = std::path::Path::new(&path_name);
    let dir = std::path::Path::new(&dir_name);

    let result = read_to_string(path);
    let string;
    match result {
        Ok(s) => {
            string = s;
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            let config = MarkovConfig {
                markov_type: MarkovType::default(),
                chance: Some(10),
            };

            let toml = toml::to_string(&config)?;
            write_default(dir, path, toml)?;
            string = read_to_string(path)?;
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

fn write_default(
    dir: &std::path::Path,
    path: &std::path::Path,
    string: String,
) -> Result<(), Error> {
    create_dir_all(dir)?;

    let mut file = File::create(path)?;
    file.write_all(string.as_bytes())?;
    Ok(())
}
