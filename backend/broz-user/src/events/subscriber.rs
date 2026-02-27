use std::sync::Arc;
use futures_lite::StreamExt;
use lapin::options::BasicAckOptions;

use broz_shared::types::event::{routing_keys, payloads, Event};

use crate::AppState;
use crate::services::profile_service;

/// Listen for auth.user.registered events to create default profiles
pub async fn listen_user_registered(state: Arc<AppState>) -> anyhow::Result<()> {
    let consumer = state.rabbitmq.subscribe(
        "broz-user.auth.user.registered",
        &[routing_keys::AUTH_USER_REGISTERED],
    ).await?;

    tracing::info!("listening for auth.user.registered events");

    let mut consumer = consumer;
    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => {
                match serde_json::from_slice::<Event<payloads::UserRegistered>>(&delivery.data) {
                    Ok(event) => {
                        let data = &event.data;
                        tracing::info!(
                            credential_id = %data.credential_id,
                            email = %data.email,
                            "received user.registered event"
                        );

                        match profile_service::create_default_profile(
                            &state.db,
                            data.credential_id,
                            &data.email,
                        ) {
                            Ok(profile) => {
                                tracing::info!(
                                    profile_id = %profile.id,
                                    "profile created for new user"
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    error = %e,
                                    credential_id = %data.credential_id,
                                    "failed to create default profile"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed to deserialize user.registered event");
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
