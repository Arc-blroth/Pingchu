//! # 🐁 Ping'chu!
//!
//! A Discord bot to track pings, made for [LePichu](https://github.com/lepichu)'s  array of servers.

#![feature(default_free_fn)]
#![feature(try_blocks)]
#![feature(yeet_expr)]
#![allow(clippy::too_many_arguments)]

pub mod commands;
pub mod config;
pub mod data;
pub mod ping;
pub mod utils;

use std::default::default;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context as AnyhowContext, Error, Result};
use poise::builtins::create_application_commands;
use poise::serenity::prelude::GatewayIntents;
use poise::serenity_prelude::{Activity, ActivityType, ApplicationCommand};
use poise::{BoxFuture, Context, Framework, FrameworkOptions};
use sea_orm::DatabaseConnection;

use crate::config::PingchuConfig;

pub struct Pingchu {
    pub config: PingchuConfig,
    pub database: DatabaseConnection,
}

pub type PingchuContext<'a> = Context<'a, Pingchu, Error>;

#[tokio::main]
pub async fn main() -> Result<()> {
    let token = read_token()?;
    let config = config::load_config();
    let database = data::load_database().await.context("Couldn't load database!")?;

    Framework::build()
        .options(FrameworkOptions {
            commands: vec![commands::pinginfo()],
            command_check: Some(allow_on_server),
            listener: ping::ping_listener,
            ..default()
        })
        .token(token)
        .intents(GatewayIntents::non_privileged().union(GatewayIntents::MESSAGE_CONTENT))
        .user_data_setup(move |ctx, ready, framework| {
            Box::pin(async move {
                ApplicationCommand::set_global_application_commands(&ctx.http, |x| {
                    *x = create_application_commands(&framework.options().commands);
                    x
                })
                .await?;

                ctx.shard.set_activity(Some(match config.status_type {
                    ActivityType::Listening => Activity::listening(config.status.clone()),
                    ActivityType::Watching => Activity::watching(config.status.clone()),
                    _ => Activity::playing(config.status.clone()),
                }));

                println!(
                    "Whomst pinged @everyone? Logged in as `{}#{}`!",
                    ready.user.name, ready.user.discriminator
                );

                Ok(Pingchu { config, database })
            })
        })
        .run()
        .await
        .context("Pingchu crashed :(")
}

fn read_token() -> Result<String> {
    // search through env variables first
    return if let Ok(token) = std::env::var("TOKEN") {
        Ok(token)
    } else if let Ok(token) = std::env::var("PINGCHU_TOKEN") {
        Ok(token)
    } else {
        // read from .token if possible
        let path = Path::new(".token");
        if path.exists() {
            fs::read_to_string(path).context("Could not start the bot")
        } else {
            do yeet anyhow!("Could not start the bot: missing token!");
        }
    };
}

fn allow_on_server(ctx: PingchuContext<'_>) -> BoxFuture<Result<bool>> {
    Box::pin(async move {
        match ctx.guild_id() {
            Some(guild) => Ok(ctx.data().config.allowed_servers.contains_key(&guild)),
            // pingchu makes no sense in DMs
            None => Ok(false),
        }
    })
}
