use super::super::super::markov::{DEFAULT_MARKOV_TYPE, DEFAULT_REPLY_MODE};
use super::{
    chat, Access, AccessConfig, AdminCmdAccess, AdminCmdAccessConfig, MarkovAccess,
    MarkovAccessConfig, MarkovConfig, MarkovConfigToml,
};

pub const DEFAULT_CHANCE: u64 = 10;

pub const DEFAULT_MARKOV_ACCESS_APPEND: chat::Access = chat::Access::All;
pub const DEFAULT_MARKOV_ACCESS_GENERATE: chat::Access = chat::Access::All;
pub const DEFAULT_MARKOV_ACCESS_REPLY: chat::Access = chat::Access::All;

pub const DEFAULT_MARKOV_ACCESS: MarkovAccessConfig = MarkovAccessConfig {
    append: DEFAULT_MARKOV_ACCESS_APPEND,
    generate: DEFAULT_MARKOV_ACCESS_GENERATE,
    reply: DEFAULT_MARKOV_ACCESS_REPLY,
};

pub const DEFAULT_ADMIN_CMD_ACCESS_CONFIG: chat::Access = chat::Access::Admins;
pub const DEFAULT_ADMIN_CMD_ACCESS_BLACKLIST: chat::Access = chat::Access::Admins;

pub const DEFAULT_ADMIN_CMD_ACCESS: AdminCmdAccessConfig = AdminCmdAccessConfig {
    config: DEFAULT_ADMIN_CMD_ACCESS_CONFIG,
    blacklist: DEFAULT_ADMIN_CMD_ACCESS_BLACKLIST,
};

pub const DEFAULT_ACCESS: AccessConfig = AccessConfig {
    markov: DEFAULT_MARKOV_ACCESS,
    admin_commands: DEFAULT_ADMIN_CMD_ACCESS,
};

pub const DEFAULT_CONFIG: MarkovConfig = MarkovConfig {
    markov_type: DEFAULT_MARKOV_TYPE,
    chance: DEFAULT_CHANCE,
    reply_mode: DEFAULT_REPLY_MODE,
    access: DEFAULT_ACCESS,
};

pub const DEFAULT_MARKOV_ACCESS_TOML: MarkovAccess = MarkovAccess {
    append: Some(DEFAULT_MARKOV_ACCESS_APPEND),
    generate: Some(DEFAULT_MARKOV_ACCESS_GENERATE),
    reply: Some(DEFAULT_MARKOV_ACCESS_REPLY),
};

pub const DEFAULT_ADMIN_CMD_ACCESS_TOML: AdminCmdAccess = AdminCmdAccess {
    config: Some(DEFAULT_ADMIN_CMD_ACCESS_CONFIG),
    blacklist: Some(DEFAULT_ADMIN_CMD_ACCESS_BLACKLIST),
};

pub const DEFAULT_ACCESS_TOML: Access = Access {
    markov: Some(DEFAULT_MARKOV_ACCESS_TOML),
    admin_commands: Some(DEFAULT_ADMIN_CMD_ACCESS_TOML),
};

pub const DEFAULT_CONFIG_TOML: MarkovConfigToml = MarkovConfigToml {
    markov_type: Some(DEFAULT_MARKOV_TYPE),
    chance: Some(DEFAULT_CHANCE),
    reply_mode: Some(DEFAULT_REPLY_MODE),
    access: Some(DEFAULT_ACCESS_TOML),
};
