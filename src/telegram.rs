use super::database::SqliteDB;
use super::markov::Markov;

use rand::{thread_rng, Rng};

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
}

#[derive(Clone, Default)]
enum State {
    _Start,
    #[default]
    Listen,
}

async fn start_bot() -> Result<Bot, Box<dyn std::error::Error + Send + Sync>> {
    let secret = config::get_secret()?;
    Ok(Bot::new(secret.token))
}

fn chance(number: u64) -> bool {
    let mut rng = thread_rng();
    if rng.gen_range(1..number) == 1 {
        return true;
    }
    false
}

fn connect_database(chat_id: &str) -> Result<SqliteDB, Box<dyn std::error::Error + Send + Sync>> {
    let flags = sqlite::OpenFlags::new()
        .set_create()
        .set_full_mutex()
        .set_read_write();

    let dir_name = format!("./{}/", chat_id);
    let path_name = format!("./{}/model.db", chat_id);
    let path = std::path::Path::new(&path_name);
    let dir = std::path::Path::new(&dir_name);

    std::fs::create_dir_all(dir)?;
    let database = SqliteDB::new(path, flags)?;

    Ok(database)
}

type MyDialogue = Dialogue<State, dialogue::InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

async fn listen(bot: Bot, msg: Message) -> HandlerResult {
    let chat_id = msg.chat.id.0.to_string();
    let database = connect_database(&chat_id)?;

    if let Some(text) = msg.text() {
        //markov.append_line(text.into())?;
        Markov::new(Box::new(database))?.append_line(text.to_string())?;
    }

    let database = connect_database(&chat_id)?;
    let config = config::get_config(&chat_id)?;

    if chance(config.chance.unwrap()) {
        let sentence = Markov::builder(Box::new(database))
            .markov_type(config.markov_type)
            .build()?
            .generate()?;
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
    let chat_id = msg.chat.id.0.to_string();
    let database = connect_database(&chat_id)?;
    let config = config::get_config(&chat_id)?;

    let sentence = Markov::builder(Box::new(database))
        .markov_type(config.markov_type)
        .build()?
        .generate();

    match sentence {
        Ok(text) => {
            bot.send_message(msg.chat.id, text).await?;
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(e);
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
