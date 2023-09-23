use super::markov::{sneedov_append_line, sneedov_generate};
use rand::{thread_rng, Rng};
use teloxide::dispatching::{dialogue, UpdateHandler};
use teloxide::prelude::*;
use teloxide_macros::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Help,
    Start,
    Markov,
}

#[derive(Clone, Default)]
enum State {
    #[default]
    Start,
    Listen,
}

async fn start_bot() -> Bot {
    //pretty_env_logger::init();

    Bot::from_env()
}

fn chance(number: usize) -> bool {
    let mut rng = thread_rng();
    if rng.gen_range(1..number) == 1 {
        return true;
    }
    false
}

fn connect_database(chat_id: &str) -> Result<sqlite::Connection, sqlite::Error> {
    let flags = sqlite::OpenFlags::new()
        .set_create()
        .set_full_mutex()
        .set_read_write();
    let path_name = format!("./{d}.db", d = chat_id);
    let path = std::path::Path::new(&path_name);
    let connection = sqlite::Connection::open_with_flags(path, flags)?;

    Ok(connection)
}

type MyDialogue = Dialogue<State, dialogue::InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

async fn listen(bot: Bot, msg: Message) -> HandlerResult {
    let chat_id = msg.chat.id.0.to_string();
    let database = connect_database(&chat_id)?;

    if let Some(text) = msg.text() {
        sneedov_append_line(&database, text)?;
    }

    if chance(10) {
        let sentence = sneedov_generate(&database)?;
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
    let sentence = sneedov_generate(&database)?;

    bot.send_message(msg.chat.id, sentence).await?;
    Ok(())
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(
            case![State::Start]
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
    let bot = start_bot().await;

    Dispatcher::builder(bot, schema())
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
