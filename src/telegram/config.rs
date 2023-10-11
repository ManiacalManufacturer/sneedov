use super::super::markov::{MarkovType, ReplyMode};
use super::chat;
use serde::{Deserialize, Serialize};
use tokio::fs::{create_dir_all, read_to_string, File};
use tokio::io::AsyncWriteExt;
use toml;

pub mod defaults;
use defaults::*;

#[derive(Serialize, Deserialize)]
pub struct Secret {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct MarkovAccess {
    append: Option<chat::Access>,
    generate: Option<chat::Access>,
    reply: Option<chat::Access>,
}

#[derive(Serialize, Deserialize)]
pub struct AdminCmdAccess {
    config: Option<chat::Access>,
    blacklist: Option<chat::Access>,
}

#[derive(Serialize, Deserialize)]
pub struct Access {
    markov: Option<MarkovAccess>,
    admin_commands: Option<AdminCmdAccess>,
}

#[derive(Serialize, Deserialize)]
pub struct MarkovConfigToml {
    markov_type: Option<MarkovType>,
    chance: Option<u64>,
    reply_mode: Option<ReplyMode>,
    separate_newline: Option<bool>,
    access: Option<Access>,
}

#[derive(Serialize, Deserialize)]
pub struct MarkovAccessConfig {
    pub append: chat::Access,
    pub generate: chat::Access,
    pub reply: chat::Access,
}

#[derive(Serialize, Deserialize)]
pub struct AdminCmdAccessConfig {
    pub config: chat::Access,
    pub blacklist: chat::Access,
}

#[derive(Serialize, Deserialize)]
pub struct AccessConfig {
    pub markov: MarkovAccessConfig,
    pub admin_commands: AdminCmdAccessConfig,
}

pub struct MarkovConfig {
    pub markov_type: MarkovType,
    pub chance: u64,
    pub reply_mode: ReplyMode,
    pub separate_newline: bool,
    pub access: AccessConfig,
}

type Error = Box<dyn std::error::Error + Send + Sync>;

macro_rules! get_or_default {
    ($bool:ident, $key:expr, $val:expr) => {{
        match $key {
            Some(v) => v.clone(),
            None => {
                $bool = true;
                let v = $val;
                $key = Some(v);
                v
            }
        }
    }};
}

async fn set_missing_config(
    configtoml: &mut MarkovConfigToml,
    path: &std::path::Path,
) -> Result<MarkovConfig, Error> {
    let mut has_missing = false;
    let chance = get_or_default!(has_missing, configtoml.chance, 10);
    let markov_type = get_or_default!(has_missing, configtoml.markov_type, MarkovType::default());
    let reply_mode = get_or_default!(has_missing, configtoml.reply_mode, ReplyMode::default());
    let separate_newline = get_or_default!(has_missing, configtoml.separate_newline, true);
    //SCHIZOPHRENIC CODE!!!
    let access = match &mut configtoml.access {
        Some(v) => {
            let markov = match &mut v.markov {
                Some(v) => MarkovAccessConfig {
                    generate: get_or_default!(
                        has_missing,
                        v.generate,
                        DEFAULT_MARKOV_ACCESS_GENERATE
                    ),
                    append: get_or_default!(has_missing, v.append, DEFAULT_MARKOV_ACCESS_APPEND),
                    reply: get_or_default!(has_missing, v.reply, DEFAULT_MARKOV_ACCESS_REPLY),
                },
                None => {
                    has_missing = true;
                    v.markov = Some(DEFAULT_MARKOV_ACCESS_TOML);
                    DEFAULT_MARKOV_ACCESS
                }
            };

            let admin_commands = match &mut v.admin_commands {
                Some(v) => AdminCmdAccessConfig {
                    config: get_or_default!(has_missing, v.config, DEFAULT_ADMIN_CMD_ACCESS_CONFIG),
                    blacklist: get_or_default!(
                        has_missing,
                        v.blacklist,
                        DEFAULT_ADMIN_CMD_ACCESS_BLACKLIST
                    ),
                },
                None => {
                    has_missing = true;
                    v.admin_commands = Some(DEFAULT_ADMIN_CMD_ACCESS_TOML);
                    DEFAULT_ADMIN_CMD_ACCESS
                }
            };

            AccessConfig {
                markov,
                admin_commands,
            }
        }
        None => {
            has_missing = true;
            configtoml.access = Some(DEFAULT_ACCESS_TOML);
            DEFAULT_ACCESS
        }
    };
    //TODO: Change all of this to some recursive macro

    if has_missing {
        write_missing(&toml::to_string(&configtoml)?, path).await?;
    }

    Ok(MarkovConfig {
        chance,
        markov_type,
        reply_mode,
        separate_newline,
        access,
    })
}

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
            let config = DEFAULT_CONFIG_TOML;

            let toml = toml::to_string(&config)?;
            write_default(dir, path, &toml).await?;
            //string = read_to_string(path).await?;
            return Ok(DEFAULT_CONFIG);
        }
        Err(e) => {
            return Err(Box::new(e));
        }
    };

    let mut config: MarkovConfigToml = toml::from_str(&string)?;

    Ok(set_missing_config(&mut config, path).await?)
}

async fn write_missing(string: &str, path: &std::path::Path) -> Result<(), Error> {
    let mut file = File::create(path).await?;
    file.write_all(string.as_bytes()).await?;
    Ok(())
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
