use super::database::SqliteDB;
use super::markov::Markov;

use std::sync::Arc;
use teloxide::dispatching::{dialogue, UpdateHandler};
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use teloxide_macros::BotCommands;

pub mod chat;
pub mod config;

use chat::{get_user_level, match_user_levels};
use config::MarkovConfig;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "Display this text")]
    Help,
    #[command(description = "Generate a sentence")]
    Markov,
    #[command(description = "Generate a reply sentence without appending")]
    Reply(String),
    #[command(description = "Blacklist a user")]
    Blacklist,
    #[command(description = "Unblacklist a user")]
    Unblacklist,
}

#[derive(Clone, Default)]
enum State {
    #[default]
    Listen,
}

async fn start_bot() -> Result<Bot, Box<dyn std::error::Error + Send + Sync>> {
    let secret = config::get_secret().await?;
    Ok(Bot::new(secret.token))
}

async fn get_bot_id() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let secret = config::get_secret().await?;
    if let Some(tuple) = secret.token.split_once(':') {
        Ok(tuple.0.to_string())
    } else {
        let err: Box<dyn std::error::Error + Send + Sync> =
            String::from("Invalid Token: Could not get bot id from token").into();
        Err(err)
    }
}

async fn connect_database(
    chat_id: &str,
) -> Result<SqliteDB, Box<dyn std::error::Error + Send + Sync>> {
    let dir_name = format!("./{}/", chat_id);
    let path_name = format!("./{}/model.db", chat_id);
    let path = std::path::Path::new(&path_name);
    let dir = std::path::Path::new(&dir_name);

    std::fs::create_dir_all(dir)?;
    let database = SqliteDB::new(path).await?;

    Ok(database)
}

async fn create_markov(
    chat_id: &str,
    config: &MarkovConfig,
) -> Result<Markov, Box<dyn std::error::Error + Send + Sync>> {
    let database = Arc::new(connect_database(chat_id).await?);

    Markov::builder(database)
        .markov_type(config.markov_type)
        .markov_chance(config.chance)
        .reply_mode(config.reply_mode)
        .build()
        .await
}

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

async fn listen(bot: Bot, msg: Message) -> HandlerResult {
    //let chat_id = msg.chat.id.0.to_string();
    //let database = connect_database(&chat_id).await?;
    let chat_id = &msg.chat.id.to_string();
    let config = config::get_config(chat_id).await?;
    let markov = create_markov(chat_id, &config).await?;

    let user_level = get_user_level(
        bot.get_chat_member(
            msg.chat.id,
            msg.from().expect("Must be MessageKind::Common").id,
        )
        .await?,
        msg.chat.id,
    )
    .await?;

    let bot_id = get_bot_id().await?;
    if let Some(text) = msg.text() {
        if user_level.is_authorized(config.access.markov.append) {
            if config.separate_newline {
                if let Err(e) = markov.append_newlines(text).await {
                    eprintln!("Couldn't append to database: {}", e);
                    return Err(e);
                }
            } else {
                if let Err(e) = markov.append_line(text).await {
                    eprintln!("Couldn't append to database: {}", e);
                    return Err(e);
                }
            }
        }
    }

    if let Some(reply) = msg.reply_to_message() {
        if reply.from().unwrap().id.to_string() == bot_id {
            if user_level.is_authorized(config.access.markov.reply) {
                if let Some(text) = msg.text() {
                    let sentence = markov.generate_reply(text).await?;
                    bot.send_message(msg.chat.id, sentence)
                        .reply_to_message_id(msg.id)
                        .await?;
                    return Ok(());
                }
            }
        }
    }
    if !user_level.is_authorized(config.access.markov.generate) {
        return Ok(());
    }

    if markov.chance() {
        let sentence = markov.generate().await?;
        bot.send_message(msg.chat.id, sentence).await?;
    }

    Ok(())
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .reply_to_message_id(msg.id)
        .await?;
    Ok(())
}

async fn generate(bot: Bot, msg: Message) -> HandlerResult {
    let chat_id = &msg.chat.id.to_string();
    let config = config::get_config(chat_id).await?;
    let markov = create_markov(chat_id, &config).await?;

    let from = bot
        .get_chat_member(
            msg.chat.id,
            msg.from().expect("Must be MessageKind::Common").id,
        )
        .await?;
    if !get_user_level(from, msg.chat.id)
        .await?
        .is_authorized(config.access.markov.generate)
    {
        bot.send_message(
            msg.chat.id,
            format!(
                "You do not have permission to use this command! (Access level: {})",
                config.access.markov.generate
            ),
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }

    let sentence = markov.generate().await;

    match sentence {
        Ok(text) => {
            bot.send_message(msg.chat.id, text).await?;
            Ok(())
        }
        Err(e) => {
            eprintln!("{}", e);
            Err(e)
        }
    }
}

async fn reply(bot: Bot, msg: Message, cmd: Command) -> HandlerResult {
    let chat_id = &msg.chat.id.to_string();
    let config = config::get_config(&chat_id).await?;

    let from = bot
        .get_chat_member(
            msg.chat.id,
            msg.from().expect("Must be MessageKind::Common").id,
        )
        .await?;
    if !get_user_level(from, msg.chat.id)
        .await?
        .is_authorized(config.access.markov.reply)
    {
        bot.send_message(
            msg.chat.id,
            format!(
                "You do not have permission to use this command! (Access level: {})",
                config.access.markov.reply
            ),
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }

    let markov = create_markov(chat_id, &config).await?;
    let sentence;

    if let Command::Reply(text) = cmd {
        if !text.is_empty() {
            sentence = markov.generate_reply(&text).await;

            match sentence {
                Ok(text) => {
                    bot.send_message(msg.chat.id, text).await?;
                }
                Err(e) => {
                    eprintln!("{}", e);
                    return Err(e);
                }
            }
        } else {
            bot.send_message(msg.chat.id, "The command was empty!")
                .reply_to_message_id(msg.id)
                .await?;
        }
    }

    Ok(())
}

async fn blacklist(bot: Bot, msg: Message) -> HandlerResult {
    let chat_id = &msg.chat.id.to_string();
    let config = config::get_config(&chat_id).await?;

    let user_level = get_user_level(
        bot.get_chat_member(
            msg.chat.id,
            msg.from().expect("Must be MessageKind::Common").id,
        )
        .await?,
        msg.chat.id,
    )
    .await?;

    if !user_level.is_authorized(config.access.admin_commands.blacklist) {
        bot.send_message(
            msg.chat.id,
            format!(
                "You do not have permission to use this command! (Access level: {})",
                config.access.admin_commands.blacklist
            ),
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }

    match msg.reply_to_message() {
        Some(replied) => {
            let chat_member = bot
                .get_chat_member(
                    msg.chat.id,
                    replied.from().expect("Must be MessageKind::Common").id,
                )
                .await?;
            let other_level = get_user_level(chat_member, msg.chat.id).await?;

            match match_user_levels(user_level, other_level) {
                Ok(_) => {
                    chat::blacklist(
                        bot.get_chat_member(
                            msg.chat.id,
                            replied.from().expect("Must be MessageKind::Common").id,
                        )
                        .await?,
                        msg.chat.id,
                    )
                    .await?;
                    bot.send_message(msg.chat.id, "User has been blacklisted")
                        .reply_to_message_id(msg.id)
                        .await?;
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, e)
                        .reply_to_message_id(msg.id)
                        .await?;
                }
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Use this command as a reply on another user")
                .await?;
        }
    }
    Ok(())
}

async fn unblacklist(bot: Bot, msg: Message) -> HandlerResult {
    let chat_id = &msg.chat.id.to_string();
    let config = config::get_config(&chat_id).await?;

    let user_level = get_user_level(
        bot.get_chat_member(
            msg.chat.id,
            msg.from().expect("Must be MessageKind::Common").id,
        )
        .await?,
        msg.chat.id,
    )
    .await?;

    if !user_level.is_authorized(config.access.admin_commands.blacklist) {
        bot.send_message(
            msg.chat.id,
            format!(
                "You do not have permission to use this command! (Access level: {})",
                config.access.admin_commands.blacklist
            ),
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }

    match msg.reply_to_message() {
        Some(replied) => {
            let chat_member = bot
                .get_chat_member(
                    msg.chat.id,
                    replied.from().expect("Must be MessageKind::Common").id,
                )
                .await?;
            let other_level = get_user_level(chat_member, msg.chat.id).await?;

            match match_user_levels(user_level, other_level) {
                Ok(_) => {
                    chat::unblacklist(
                        bot.get_chat_member(
                            msg.chat.id,
                            replied.from().expect("Must be MessageKind::Common").id,
                        )
                        .await?,
                        msg.chat.id,
                    )
                    .await?;
                    bot.send_message(msg.chat.id, "User has been unblacklisted")
                        .reply_to_message_id(msg.id)
                        .await?;
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, e)
                        .reply_to_message_id(msg.id)
                        .await?;
                }
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Use this command as a reply on another user")
                .await?;
        }
    }
    Ok(())
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>().branch(
        case![State::Listen]
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Markov].endpoint(generate))
            .branch(case![Command::Blacklist].endpoint(blacklist))
            .branch(case![Command::Unblacklist].endpoint(unblacklist))
            .branch(case![Command::Reply(text)])
            .endpoint(reply),
    );
    //.branch(case![State::Listen].branch(case![Command::Reply(text)].endpoint(reply)));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::Listen].endpoint(listen));

    dialogue::enter::<Update, dialogue::InMemStorage<State>, State, _>().branch(message_handler)
}

pub async fn start_dispatcher() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let bot = start_bot().await?;

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![dialogue::InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
