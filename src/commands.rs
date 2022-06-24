use poise::serenity_prelude::{Colour, CreateEmbed, Message};

pub const EMBED_COLOR: Colour = Colour(0xEF5858);

/// Applies a standard "UI" theme to embeds sent by Pingchu.
pub fn apply_ui(embed: &mut CreateEmbed, in_reply_to: Option<&Message>) {
    embed.color(EMBED_COLOR);
    if let Some(message) = in_reply_to {
        embed.footer(|footer| {
            footer.text("Ping'chu!").icon_url(
                message
                    .author
                    .avatar_url()
                    .unwrap_or_else(|| message.author.default_avatar_url()),
            )
        });
        embed.timestamp(message.timestamp);
    }
}
