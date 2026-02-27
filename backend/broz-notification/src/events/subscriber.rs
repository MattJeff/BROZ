use std::sync::Arc;

use futures_lite::StreamExt;
use lapin::options::BasicAckOptions;

use broz_shared::types::event::{payloads, routing_keys, Event};

use crate::services::notification_service;
use crate::AppState;

/// Listen for follow events (follow.requested, follow.accepted).
pub async fn listen_follow_events(state: Arc<AppState>) -> anyhow::Result<()> {
    let mut consumer = state.rabbitmq.subscribe(
        "broz-notification.follow",
        &[
            routing_keys::USER_FOLLOW_REQUESTED,
            routing_keys::USER_FOLLOW_ACCEPTED,
        ],
    ).await?;

    tracing::info!("listening for follow events");

    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => {
                let routing_key = delivery.routing_key.to_string();

                if routing_key == routing_keys::USER_FOLLOW_REQUESTED {
                    match serde_json::from_slice::<Event<payloads::FollowRequested>>(&delivery.data) {
                        Ok(event) => {
                            let data = &event.data;
                            tracing::info!(
                                follower_id = %data.follower_id,
                                following_id = %data.following_id,
                                "received follow.requested event"
                            );

                            if let Err(e) = notification_service::create_notification(
                                &state.db,
                                data.following_id,
                                "follow_requested",
                                "New follow request",
                                &format!("{} wants to follow you", data.follower_display_name),
                                Some(serde_json::json!({
                                    "follower_id": data.follower_id,
                                    "follower_display_name": data.follower_display_name,
                                })),
                            ) {
                                tracing::error!(error = %e, "failed to create follow_requested notification");
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "failed to deserialize follow.requested event");
                        }
                    }
                } else if routing_key == routing_keys::USER_FOLLOW_ACCEPTED {
                    match serde_json::from_slice::<Event<payloads::FollowAccepted>>(&delivery.data) {
                        Ok(event) => {
                            let data = &event.data;
                            tracing::info!(
                                follower_id = %data.follower_id,
                                following_id = %data.following_id,
                                "received follow.accepted event"
                            );

                            if let Err(e) = notification_service::create_notification(
                                &state.db,
                                data.follower_id,
                                "follow_accepted",
                                "Follow request accepted",
                                "Your follow request was accepted",
                                Some(serde_json::json!({
                                    "following_id": data.following_id,
                                })),
                            ) {
                                tracing::error!(error = %e, "failed to create follow_accepted notification");
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "failed to deserialize follow.accepted event");
                        }
                    }
                }

                let _ = delivery.ack(BasicAckOptions::default()).await;
            }
            Err(e) => {
                tracing::error!(error = %e, "follow consumer error");
            }
        }
    }

    Ok(())
}

/// Listen for like events (like.sent).
pub async fn listen_like_events(state: Arc<AppState>) -> anyhow::Result<()> {
    let mut consumer = state.rabbitmq.subscribe(
        "broz-notification.like.sent",
        &[routing_keys::USER_LIKE_SENT],
    ).await?;

    tracing::info!("listening for like events");

    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => {
                match serde_json::from_slice::<Event<payloads::LikeSent>>(&delivery.data) {
                    Ok(event) => {
                        let data = &event.data;
                        tracing::info!(
                            liker_id = %data.liker_id,
                            liked_id = %data.liked_id,
                            "received like.sent event"
                        );

                        if let Err(e) = notification_service::create_notification(
                            &state.db,
                            data.liked_id,
                            "like_received",
                            "Someone liked you!",
                            &format!("{} liked you", data.liker_display_name),
                            Some(serde_json::json!({
                                "liker_id": data.liker_id,
                                "liker_display_name": data.liker_display_name,
                                "match_session_id": data.match_session_id,
                            })),
                        ) {
                            tracing::error!(error = %e, "failed to create like notification");
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed to deserialize like.sent event");
                    }
                }

                let _ = delivery.ack(BasicAckOptions::default()).await;
            }
            Err(e) => {
                tracing::error!(error = %e, "like consumer error");
            }
        }
    }

    Ok(())
}

/// Listen for message events (message.sent).
pub async fn listen_message_events(state: Arc<AppState>) -> anyhow::Result<()> {
    let mut consumer = state.rabbitmq.subscribe(
        "broz-notification.message.sent",
        &[routing_keys::MESSAGING_MESSAGE_SENT],
    ).await?;

    tracing::info!("listening for message events");

    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => {
                match serde_json::from_slice::<Event<payloads::MessageSent>>(&delivery.data) {
                    Ok(event) => {
                        let data = &event.data;
                        tracing::info!(
                            sender_id = %data.sender_id,
                            conversation_id = %data.conversation_id,
                            "received message.sent event"
                        );

                        // The event user_id (set by the publisher) is typically the sender.
                        // We create a notification for the conversation participants.
                        // Since we don't have the full participant list in the payload,
                        // we create a notification using the event's user_id context.
                        // In practice, the publisher should include recipient IDs or
                        // we query the conversation members. For now, we use event.user_id
                        // as the recipient if available, otherwise skip.
                        if let Some(recipient_id) = event.user_id {
                            // Only notify if the recipient is not the sender
                            if recipient_id != data.sender_id {
                                if let Err(e) = notification_service::create_notification(
                                    &state.db,
                                    recipient_id,
                                    "message_received",
                                    "New message",
                                    &format!("New message from {}", data.sender_display_name),
                                    Some(serde_json::json!({
                                        "conversation_id": data.conversation_id,
                                        "message_id": data.message_id,
                                        "sender_id": data.sender_id,
                                        "sender_display_name": data.sender_display_name,
                                        "content_preview": data.content_preview,
                                    })),
                                ) {
                                    tracing::error!(error = %e, "failed to create message notification");
                                }
                            }
                        } else {
                            tracing::warn!(
                                conversation_id = %data.conversation_id,
                                "message.sent event missing user_id, skipping notification"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed to deserialize message.sent event");
                    }
                }

                let _ = delivery.ack(BasicAckOptions::default()).await;
            }
            Err(e) => {
                tracing::error!(error = %e, "message consumer error");
            }
        }
    }

    Ok(())
}

/// Listen for sanction events (sanction.issued, sanction.lifted).
pub async fn listen_sanction_events(state: Arc<AppState>) -> anyhow::Result<()> {
    let mut consumer = state.rabbitmq.subscribe(
        "broz-notification.sanction",
        &[
            routing_keys::MODERATION_SANCTION_ISSUED,
            routing_keys::MODERATION_SANCTION_LIFTED,
        ],
    ).await?;

    tracing::info!("listening for sanction events");

    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => {
                let routing_key = delivery.routing_key.to_string();

                if routing_key == routing_keys::MODERATION_SANCTION_ISSUED {
                    match serde_json::from_slice::<Event<payloads::SanctionIssued>>(&delivery.data) {
                        Ok(event) => {
                            let data = &event.data;
                            tracing::info!(
                                user_id = %data.user_id,
                                sanction_type = %data.sanction_type,
                                "received sanction.issued event"
                            );

                            if let Err(e) = notification_service::create_notification(
                                &state.db,
                                data.user_id,
                                "sanction_issued",
                                "Account sanction",
                                &format!("You received a {}: {}", data.sanction_type, data.reason),
                                Some(serde_json::json!({
                                    "sanction_id": data.sanction_id,
                                    "sanction_type": data.sanction_type,
                                    "reason": data.reason,
                                    "expires_at": data.expires_at,
                                })),
                            ) {
                                tracing::error!(error = %e, "failed to create sanction_issued notification");
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "failed to deserialize sanction.issued event");
                        }
                    }
                } else if routing_key == routing_keys::MODERATION_SANCTION_LIFTED {
                    match serde_json::from_slice::<Event<payloads::SanctionLifted>>(&delivery.data) {
                        Ok(event) => {
                            let data = &event.data;
                            tracing::info!(
                                user_id = %data.user_id,
                                sanction_id = %data.sanction_id,
                                "received sanction.lifted event"
                            );

                            if let Err(e) = notification_service::create_notification(
                                &state.db,
                                data.user_id,
                                "sanction_lifted",
                                "Sanction lifted",
                                "Your sanction has been lifted",
                                Some(serde_json::json!({
                                    "sanction_id": data.sanction_id,
                                })),
                            ) {
                                tracing::error!(error = %e, "failed to create sanction_lifted notification");
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "failed to deserialize sanction.lifted event");
                        }
                    }
                }

                let _ = delivery.ack(BasicAckOptions::default()).await;
            }
            Err(e) => {
                tracing::error!(error = %e, "sanction consumer error");
            }
        }
    }

    Ok(())
}
