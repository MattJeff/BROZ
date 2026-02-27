use std::sync::Arc;
use diesel::prelude::*;
use futures_lite::StreamExt;
use lapin::options::BasicAckOptions;
use uuid::Uuid;

use broz_shared::types::event::{routing_keys, payloads, Event};

use crate::AppState;
use crate::models::{NewConversation, NewConversationMember};
use crate::schema::{conversations, conversation_members};

/// Listen for user.follow.accepted events to auto-create 1:1 conversations
pub async fn listen_follow_accepted(state: Arc<AppState>) -> anyhow::Result<()> {
    let consumer = state.rabbitmq.subscribe(
        "broz-messaging.user.follow.accepted",
        &[routing_keys::USER_FOLLOW_ACCEPTED],
    ).await?;

    tracing::info!("listening for user.follow.accepted events");

    let mut consumer = consumer;
    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => {
                match serde_json::from_slice::<Event<payloads::FollowAccepted>>(&delivery.data) {
                    Ok(event) => {
                        let data = &event.data;
                        tracing::info!(
                            follower_id = %data.follower_id,
                            following_id = %data.following_id,
                            "received follow.accepted event"
                        );

                        if let Err(e) = create_dm_if_not_exists(
                            &state.db,
                            data.follower_id,
                            data.following_id,
                        ) {
                            tracing::error!(
                                error = %e,
                                follower_id = %data.follower_id,
                                following_id = %data.following_id,
                                "failed to auto-create DM conversation"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed to deserialize follow.accepted event");
                    }
                }
                let _ = delivery.ack(BasicAckOptions::default()).await;
            }
            Err(e) => {
                tracing::error!(error = %e, "consumer error");
            }
        }
    }

    Ok(())
}

/// Create a 1:1 conversation between two users if one does not already exist.
fn create_dm_if_not_exists(
    db: &crate::DbPool,
    user_a: Uuid,
    user_b: Uuid,
) -> anyhow::Result<()> {
    let mut conn = db.get()?;

    // Check if a 1:1 (non-group) conversation already exists between these two users.
    // We look for conversations where user_a is a member AND is_group = false.
    let existing: Vec<Uuid> = conversation_members::table
        .inner_join(conversations::table)
        .filter(conversations::is_group.eq(false))
        .filter(conversation_members::user_id.eq(user_a))
        .select(conversation_members::conversation_id)
        .load::<Uuid>(&mut conn)?;

    if !existing.is_empty() {
        // Check if user_b is also a member of any of those conversations
        let shared: i64 = conversation_members::table
            .filter(conversation_members::conversation_id.eq_any(&existing))
            .filter(conversation_members::user_id.eq(user_b))
            .count()
            .get_result(&mut conn)?;

        if shared > 0 {
            tracing::debug!(
                user_a = %user_a,
                user_b = %user_b,
                "DM conversation already exists, skipping creation"
            );
            return Ok(());
        }
    }

    // Create a new 1:1 conversation
    let new_conv = NewConversation {
        is_group: false,
        group_name: None,
        group_photo_url: None,
    };

    let conv_id: Uuid = diesel::insert_into(conversations::table)
        .values(&new_conv)
        .returning(conversations::id)
        .get_result(&mut conn)?;

    // Add both users as members
    let members = vec![
        NewConversationMember {
            conversation_id: conv_id,
            user_id: user_a,
        },
        NewConversationMember {
            conversation_id: conv_id,
            user_id: user_b,
        },
    ];

    diesel::insert_into(conversation_members::table)
        .values(&members)
        .execute(&mut conn)?;

    tracing::info!(
        conversation_id = %conv_id,
        user_a = %user_a,
        user_b = %user_b,
        "auto-created DM conversation from follow.accepted"
    );

    Ok(())
}
