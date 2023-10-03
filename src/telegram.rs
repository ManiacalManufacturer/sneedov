use super::database::SqliteDB;
use super::markov::Markov;

use std::sync::Arc;

use teloxide::dispatching::{dialogue, UpdateHandler};
use teloxide::prelude::*;
use teloxide_macros::BotCommands;

pub mod config;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Help,
    Start,
    Markov,
    Reply(String),
}

#[derive(Clone, Default)]
enum State {
    _Start,
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

async fn create_markov(chat_id: &str) -> Result<Markov, Box<dyn std::error::Error + Send + Sync>> {
    let database = Arc::new(connect_database(chat_id).await?);
    let config = config::get_config(chat_id).await?;

    Markov::builder(database)
        .markov_type(config.markov_type)
        .markov_chance(config.chance)
        .reply_mode(config.reply_mode)
        .build()
        .await
}

type MyDialogue = Dialogue<State, dialogue::InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

async fn listen(bot: Bot, msg: Message) -> HandlerResult {
    //let chat_id = msg.chat.id.0.to_string();
    //let database = connect_database(&chat_id).await?;

    let markov = create_markov(&msg.chat.id.0.to_string()).await?;

    let bot_id = get_bot_id().await?;
    if let Some(text) = msg.text() {
        if let Err(e) = markov.append_line(text).await {
            eprintln!("Couldn't append to database: {}", e);
            return Err(e);
        }
    }

    if let Some(reply) = msg.reply_to_message() {
        if reply.from().unwrap().id.to_string() == bot_id {
            if let Some(text) = msg.text() {
                let sentence = markov.generate_reply(text).await?;
                bot.send_message(msg.from().unwrap().id, sentence).await?;
                return Ok(());
            }
        }
    }

    if markov.chance() {
        let sentence = markov.generate().await?;
        bot.send_message(msg.chat.id, sentence).await?;
    }

    Ok(())
}

async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Now listening to messages")
        .await?;
    dialogue.update(State::Listen).await?;
    Ok(())
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Nope!").await?;
    Ok(())
}

async fn generate(bot: Bot, msg: Message) -> HandlerResult {
    let markov = create_markov(&msg.chat.id.0.to_string()).await?;

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

async fn _reply(bot: Bot, msg: Message, cmd: Command) -> HandlerResult {
    let chat_id = msg.chat.id.0.to_string();
    let database = connect_database(&chat_id).await?;
    let config = config::get_config(&chat_id).await?;
    let sentence;

    if let Command::Reply(text) = cmd {
        if !text.is_empty() {
            sentence = Markov::builder(Arc::new(database))
                .markov_type(config.markov_type)
                .build()
                .await?
                .generate_reply(&text)
                .await;

            match sentence {
                Ok(text) => {
                    bot.send_message(msg.chat.id, text).await?;
                }
                Err(e) => {
                    eprintln!("{}", e);
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            case![State::_Start]
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Start].endpoint(start)),
        )
        .branch(case![State::Listen].branch(case![Command::Markov].endpoint(generate)));
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
