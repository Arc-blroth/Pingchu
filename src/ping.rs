use anyhow::{Context as AnyhowContext, Error, Result};
use itertools::Itertools;
use lazy_static::lazy_static;
use poise::serenity_prelude::{Context, GuildId, Member, Mentionable, Permissions, RoleId, Timestamp, UserId};
use poise::{BoxFuture, Event, FrameworkContext};
use rand::seq::SliceRandom;
use regex::Regex;
use sea_orm::entity::Iterable;
use sea_orm::prelude::DateTimeUtc;
use sea_orm::sea_query::{Expr, OnConflict, Query};
use sea_orm::{ColumnTrait, EntityName, EntityTrait, QueryFilter, QueryOrder, TransactionTrait};
use unicode_segmentation::UnicodeSegmentation;

use crate::data::{execute_query, guild_ping};
use crate::{commands, utils, Pingchu};

const EVERYONE_PING: &str = "@everyone";
const HERE_PING: &str = "@here";

lazy_static! {
    static ref ROLE_PING: Regex = Regex::new(r"<@&(\d*?)>").unwrap();
}

pub fn ping_listener<'a>(
    ctx: &'a Context,
    event: &'a Event<'a>,
    framework: FrameworkContext<'a, Pingchu, Error>,
    pingchu: &'a Pingchu,
) -> BoxFuture<'a, Result<()>> {
    Box::pin(async move {
        if let Event::Message { new_message } = event {
            match new_message.guild_id {
                Some(guild) if pingchu.config.allowed_servers.contains_key(&guild) => {
                    let content = &new_message.content;
                    let member = guild.member(&ctx.http, new_message.author.id).await?;
                    let guild_roles = guild.roles(&ctx.http).await?;

                    // note: the @everyone role has the same id as the guild (clever, Discord)
                    let member_can_ping_everyone = member
                        .roles
                        .iter()
                        .chain(std::iter::once(&RoleId(guild.0)))
                        .filter_map(|x| guild_roles.get(x))
                        .any(|role| {
                            role.has_permission(Permissions::MENTION_EVERYONE)
                                || role.has_permission(Permissions::ADMINISTRATOR)
                        });

                    let user_pings = new_message.mentions.len();
                    let role_pings = {
                        let possible_pings = (*ROLE_PING).find_iter(content);
                        if member_can_ping_everyone {
                            // user can ping all roles
                            possible_pings.map(|x| x.as_str()).unique().count()
                        } else {
                            // time to figure out which roles were actually pinged
                            possible_pings
                                .filter_map(|x| x.as_str().parse::<u64>().ok())
                                .unique()
                                .filter_map(|x| guild_roles.get(&RoleId(x)))
                                .filter(|role| role.mentionable)
                                .count()
                        }
                    };
                    // note: `new_message.mention_everyone` returns true for both @here and @everyone
                    let everyone_ping = member_can_ping_everyone && content.contains(EVERYONE_PING);
                    let here_ping = member_can_ping_everyone && content.contains(HERE_PING);

                    let pings = user_pings + role_pings + everyone_ping as usize + here_ping as usize;
                    if pings > 0 {
                        // save previous state for logging @everyone pings
                        let previous_everyone = if everyone_ping {
                            Some(everyone_ping_history(pingchu, &member).await?)
                        } else {
                            None
                        };

                        upsert_guild_ping(
                            pingchu,
                            &member,
                            new_message.timestamp,
                            pings,
                            user_pings,
                            role_pings,
                            everyone_ping,
                            here_ping,
                        )
                        .await?;

                        if let Some((last_global, last_member, last_pings)) = previous_everyone {
                            pingchu.config.allowed_servers[&guild]
                                .log_channel
                                .send_message(&ctx.http, |msg| {
                                    msg.add_embed(|embed| {
                                        commands::apply_ui(embed, Some(&new_message.author), new_message.timestamp);
                                        embed
                                            .title(format!("_{} pinged @everyone!_", member.display_name()))
                                            .url(new_message.link())
                                            .field(
                                                "Message",
                                                new_message.content.graphemes(true).take(100).collect::<String>(),
                                                false,
                                            )
                                            .field("Author", new_message.author.mention(), true)
                                            .field("Total Pings", last_pings + pings as u32, true);
                                        if let Some(time) = last_global {
                                            embed.field(
                                                "Time since last @everyone",
                                                utils::format_time_v1_duration(*new_message.timestamp - time),
                                                false,
                                            );
                                        }
                                        if let Some(time) = last_member {
                                            embed.field(
                                                format!("Time since {} last pinged @everyone", member.display_name()),
                                                utils::format_time_v1_duration(*new_message.timestamp - time),
                                                false,
                                            );
                                        }
                                        embed
                                    })
                                })
                                .await?;
                        } else if new_message.mentions.iter().any(|x| x.id == framework.bot_id) {
                            let maybe_response = pingchu.config.ping_responses.choose(&mut rand::thread_rng()).cloned();
                            if let Some(response) = maybe_response {
                                new_message
                                    .channel_id
                                    .send_message(&ctx.http, |msg| msg.reference_message(new_message).content(response))
                                    .await?;
                            }
                        }
                    }
                }
                // pingchu makes no sense in DMs
                _ => {}
            }
        }
        Ok(())
    })
}

pub async fn member_ping_info(pingchu: &Pingchu, guild: GuildId, user: UserId) -> Result<Option<guild_ping::Model>> {
    guild_ping::Entity::find_by_id((guild.0 as i64, user.0 as i64))
        .one(&pingchu.database)
        .await
        .context("Couldn't fetch member ping history")
}

async fn everyone_ping_history(
    pingchu: &Pingchu,
    member: &Member,
) -> Result<(Option<DateTimeUtc>, Option<DateTimeUtc>, u32)> {
    let last_global = guild_ping::Entity::find()
        .filter(guild_ping::Column::GuildId.eq(member.guild_id.0 as i64))
        .order_by_desc(guild_ping::Column::LastEveryonePing)
        .one(&pingchu.database)
        .await
        .context("Couldn't fetch guild ping history")?
        .and_then(|x| x.last_everyone_ping);

    let (last_member, pings) = member_ping_info(pingchu, member.guild_id, member.user.id)
        .await?
        .map(|x| (x.last_everyone_ping, x.pings))
        .unwrap_or_default();

    Ok((last_global, last_member, pings))
}

async fn upsert_guild_ping(
    pingchu: &Pingchu,
    member: &Member,
    timestamp: Timestamp,
    pings: usize,
    user_pings: usize,
    role_pings: usize,
    everyone_ping: bool,
    here_ping: bool,
) -> Result<()> {
    assert!(
        pings > 0,
        "Attempted to upsert new guild ping data when there were no pings"
    );

    let guild_id = member.guild_id.0 as i64;
    let user_id = member.user.id.0 as i64;
    let time = *timestamp;
    let last_everyone_ping = everyone_ping.then_some(time);
    let last_here_ping = here_ping.then_some(time);
    let last_role_ping = (role_pings > 0).then_some(time);
    let last_user_ping = (user_pings > 0).then_some(time);
    let pings = pings as u32;

    pingchu
        .database
        .transaction(|txn| {
            Box::pin(async move {
                // apparently sea_orm doesn't support upserts yet like wtf
                let query = Query::insert()
                    .into_table(guild_ping::Entity.table_ref())
                    .columns(guild_ping::Column::iter())
                    .values_panic([
                        guild_id.into(),
                        user_id.into(),
                        last_everyone_ping.into(),
                        last_here_ping.into(),
                        last_role_ping.into(),
                        last_user_ping.into(),
                        pings.into(),
                    ])
                    .on_conflict(
                        OnConflict::columns([guild_ping::Column::GuildId, guild_ping::Column::UserId])
                            .update_exprs({
                                let mut to_update = vec![];
                                if let Some(time) = last_everyone_ping {
                                    to_update.push((guild_ping::Column::LastEveryonePing, Expr::val(time).into()));
                                }
                                if let Some(time) = last_here_ping {
                                    to_update.push((guild_ping::Column::LastHerePing, Expr::val(time).into()));
                                }
                                if let Some(time) = last_role_ping {
                                    to_update.push((guild_ping::Column::LastRolePing, Expr::val(time).into()));
                                }
                                if let Some(time) = last_user_ping {
                                    to_update.push((guild_ping::Column::LastUserPing, Expr::val(time).into()));
                                }
                                to_update.push((
                                    guild_ping::Column::Pings,
                                    Expr::col(guild_ping::Column::Pings).add(pings),
                                ));
                                to_update
                            })
                            .to_owned(),
                    )
                    .to_owned();
                execute_query(txn, &query).await.map(|_| ())
            })
        })
        .await
        .context("Failed to upsert guild ping data")
}
