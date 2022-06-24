use anyhow::{Context as AnyhowContext, Result};
use poise::serenity_prelude::{Colour, CreateEmbed, Timestamp, User};

use crate::{ping, utils, PingchuContext};

pub const EMBED_COLOR: Colour = Colour(0xEF5858);

/// Applies a standard "UI" theme to embeds sent by Pingchu.
pub fn apply_ui(embed: &mut CreateEmbed, in_reply_to: Option<&User>, timestamp: Timestamp) {
    embed.color(EMBED_COLOR);
    if let Some(author) = in_reply_to {
        embed.footer(|footer| {
            footer
                .text("Ping'chu!")
                .icon_url(author.avatar_url().unwrap_or_else(|| author.default_avatar_url()))
        });
    }
    embed.timestamp(timestamp);
}

#[poise::command(slash_command)]
/// Get ping stats for yourself or a target user.
pub async fn pinginfo(ctx: PingchuContext<'_>, #[description = "Target user."] user: Option<User>) -> Result<()> {
    let user = user.as_ref().unwrap_or_else(|| ctx.author());
    // SAFETY: the pre-command hook filters out commands not sent in guilds
    let guild = ctx.guild_id().unwrap();
    let member = guild.member(&ctx.discord().http, user.id).await?;
    let timestamp = ctx.created_at();

    let info = ping::member_ping_info(ctx.data(), ctx.guild_id().unwrap(), user.id).await?;
    let (last_everyone_ping, last_here_ping, last_role_ping, last_user_ping, pings) = match info {
        Some(m) => (
            m.last_everyone_ping,
            m.last_here_ping,
            m.last_role_ping,
            m.last_user_ping,
            m.pings,
        ),
        None => (None, None, None, None, 0),
    };

    ctx.send(|msg| {
        msg.embed(|embed| {
            apply_ui(embed, Some(user), timestamp);
            embed
                .title(format!("{}'s Ping Stats", member.display_name()))
                .field("Total Pings", pings, false);
            if let Some(time) = last_everyone_ping {
                embed.field(
                    "Time since last @everyone",
                    utils::format_time_v1_duration(*timestamp - time),
                    true,
                );
            }
            if let Some(time) = last_here_ping {
                embed.field(
                    "Time since last @here",
                    utils::format_time_v1_duration(*timestamp - time),
                    true,
                );
            }
            if let Some(time) = last_role_ping {
                embed.field(
                    "Time since last @role",
                    utils::format_time_v1_duration(*timestamp - time),
                    true,
                );
            }
            if let Some(time) = last_user_ping {
                embed.field(
                    "Time since last @User",
                    utils::format_time_v1_duration(*timestamp - time),
                    true,
                );
            }
            embed
        })
    })
    .await
    .context("Failed to reply to /pinginfo")?;
    Ok(())
}
