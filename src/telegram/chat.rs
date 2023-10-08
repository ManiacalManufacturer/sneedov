use super::super::database::{Blacklist, SqliteBlacklist};
use serde::{Deserialize, Serialize};
use teloxide::types::{ChatId, ChatMember, UserId};

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum Access {
    All,
    Admins,
    Owner,
}

pub enum User {
    Normal,
    Blacklisted,
    Admin,
    Owner,
}

impl User {
    pub fn is_authorized(&self, access: Access) -> bool {
        match (self, access) {
            (User::Blacklisted, _) => false,
            (_, Access::All) => true,
            (User::Admin, Access::Admins) => true,
            (User::Owner, _) => true,
            (_, _) => false,
        }
    }
}

impl std::fmt::Display for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Access::All => write!(f, "All users"),
            Access::Admins => write!(f, "Admins only"),
            Access::Owner => write!(f, "Owner only"),
        }
    }
}

async fn get_database() -> Result<SqliteBlacklist, Error> {
    SqliteBlacklist::new(&std::path::Path::new("./chats.db")).await
}

async fn is_blacklisted(user: ChatMember, chat: ChatId) -> Result<bool, Error> {
    let UserId(user_id) = user.user.id;
    let ChatId(chat_id) = chat;

    get_database().await?.is_blacklisted(chat_id, user_id).await
}

pub async fn blacklist(user: ChatMember, chat: ChatId) -> Result<(), Error> {
    let UserId(user_id) = user.user.id;
    let ChatId(chat_id) = chat;

    get_database().await?.blacklist(chat_id, user_id).await
}

pub async fn unblacklist(user: ChatMember, chat: ChatId) -> Result<(), Error> {
    let UserId(user_id) = user.user.id;
    let ChatId(chat_id) = chat;

    get_database().await?.unblacklist(chat_id, user_id).await
}

pub async fn get_user_level(user: ChatMember, chat_id: ChatId) -> Result<User, Error> {
    let is_owner = user.kind.is_owner();
    let is_admin = user.kind.is_administrator();
    let is_blacklisted = is_blacklisted(user, chat_id).await?;

    match (is_owner, is_admin, is_blacklisted) {
        (true, _, _) => Ok(User::Owner),
        (_, _, true) => Ok(User::Blacklisted),
        (_, true, _) => Ok(User::Admin),
        (_, _, _) => Ok(User::Normal),
    }
}

pub fn match_user_levels(user1: User, user2: User) -> Result<(), &'static str> {
    match (user1, user2) {
        (_, User::Owner) => Err("You cannot blacklist the owner!"),
        (User::Owner, _) => Ok(()),
        (User::Admin, User::Admin) => Err("You cannot blacklist other admins!"),
        (User::Admin, _) => Ok(()),
        (_, User::Admin) => Err("You cannot blacklist admins!"),
        (User::Normal, User::Normal) => Err("You cannot blacklist users with the same level!"),
        (User::Normal, User::Blacklisted) => Ok(()),
        (User::Blacklisted, _) => Err("You are blacklisted!"),
    }
}
