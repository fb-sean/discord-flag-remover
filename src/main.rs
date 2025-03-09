use dotenv::dotenv;
use regex::Regex;
use std::env;
use std::error::Error;
use std::sync::Arc;
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard, ShardId, StreamExt as _};
use twilight_http::Client as HttpClient;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::channel::message::EmojiReactionType;
use twilight_model::id::Id;
use twilight_model::id::marker::ChannelMarker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let token = env::var("TOKEN").expect("TOKEN is not set");

    let mut shard = Shard::new(
        ShardId::ONE,
        token.clone(),
        Intents::GUILD_MESSAGE_REACTIONS,
    );

    let http = Arc::new(HttpClient::new(token));

    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let Ok(event) = item else {
            tracing::warn!(source = ?item.unwrap_err(), "error receiving event");

            continue;
        };

        tokio::spawn(handle_event(event, Arc::clone(&http)));
    }

    Ok(())
}

async fn handle_event(
    event: Event,
    http: Arc<HttpClient>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let allowed_channels: Vec<Id<ChannelMarker>> = env::var("CHANNEL_LIST")
        .unwrap_or_default()
        .split(',')
        .filter_map(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                trimmed.parse::<Id<ChannelMarker>>().ok()
            }
        })
        .collect();

    let flag_regex = Regex::new(r"^[\u{1F1E6}-\u{1F1FF}]{2}$")?;

    match event {
        Event::ReactionAdd(reaction) if true => {
            if !allowed_channels.is_empty() && !allowed_channels.contains(&reaction.channel_id) {
                return Ok(());
            }

            if let EmojiReactionType::Unicode { name } = &reaction.emoji {
                if flag_regex.is_match(&name) {
                    tracing::info!("Flag emoji detected: {}", name);
                    if let Err(e) = http
                        .delete_all_reaction(
                            reaction.channel_id,
                            reaction.message_id,
                            &RequestReactionType::Unicode {
                                name: &*name.clone(),
                            },
                        )
                        .await
                    {
                        tracing::error!("Error adding reaction: {:?}", e);
                    } else {
                        tracing::info!(
                            "Successfully removed reaction for message {} from {}",
                            reaction.message_id,
                            reaction.user_id
                        );
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}
