use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::{env, fs};

use anyhow::Result;
use poise::serenity_prelude::{ActivityType, ChannelId, GuildId};
use serde::{Deserialize, Serialize};

pub const CONFIG_FILE: &str = ".data/config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PingchuConfig {
    pub status: String,
    pub status_type: ActivityType,
    pub allowed_servers: HashMap<GuildId, ServerConfig>,
    pub ping_responses: Vec<String>,
    pub uwu_chance: f64,
}

impl Default for PingchuConfig {
    fn default() -> Self {
        Self {
            status: "red bubbles".to_string(),
            status_type: ActivityType::Watching,
            allowed_servers: HashMap::new(),
            ping_responses: [
                "Hello!",
                "Heyo!",
                "Hi!",
                "Hai!",
                "Hewo!",
                "Hey there, I'm Ping'chu!",
                "@\u{200B}everyone",
                "Did you just ping me?",
                "WHOMST DARES PING ME",
                "why ping",
                "THAT TICKLES",
                "hehe~",
                "I've been booped!",
                "beep boop",
                "Can't catch me!",
                "_Ping'chu uses Thunderbolt! It was very effective!_",
                "_Ping'chu uses Thunderbolt! It wasn't very effective..._",
            ]
            .iter()
            .map(|x| x.to_string())
            .collect(),
            uwu_chance: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub log_channel: ChannelId,
}

pub fn load_config() -> PingchuConfig {
    let config_file = env::current_dir().unwrap().join(Path::new(CONFIG_FILE));

    // Read config from disk
    let config = if config_file.exists() {
        let result: Result<PingchuConfig> = try { serde_json::from_reader(File::open(&config_file)?)? };
        match result {
            Ok(config) => config,
            Err(err) => {
                eprintln!(
                    "Couldn't load {} config, using defaults: {}",
                    &config_file.display(),
                    err
                );
                PingchuConfig::default()
            }
        }
    } else {
        PingchuConfig::default()
    };

    // Write config back to disk
    // This ensures new properties are represented in the config file
    let write_result: Result<()> = try {
        fs::create_dir_all(&config_file.parent().unwrap())?;
        serde_json::to_writer_pretty(File::create(&config_file)?, &config)?
    };
    if let Err(err) = write_result {
        eprintln!("Couldn't update {} config: {}", &config_file.display(), err);
    }

    config
}
