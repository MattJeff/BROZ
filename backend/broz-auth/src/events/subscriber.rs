use std::sync::Arc;
use futures_lite::StreamExt;
use lapin::options::BasicAckOptions;

use broz_shared::types::event::{routing_keys, payloads, Event};

use crate::AppState;

/// Listen for sanction.issued events to update is_banned on credentials
pub async fn listen_sanction_issued(state: Arc<AppState>) -> anyhow::Result<()> {
    let consumer = state.rabbitmq.subscribe(
        "broz-auth.sanction.issued",
        &[routing_keys::MODERATION_SANCTION_ISSUED],
    ).await?;

    tracing::info!("listening for sanction.issued events");

    let mut consumer = consumer;
    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => {
                match serde_json::from_slice::<Event<payloads::SanctionIssued>>(&delivery.data) {
                    Ok(event) => {
                        let data = &event.data;
                        tracing::info!(
                            user_id = %data.user_id,
                            sanction_type = %data.sanction_type,
                            "received sanction.issued event"
                        );

                        let mut conn = match state.db.get() {
                            Ok(c) => c,
                            Err(e) => {
                                tracing::error!(error = %e, "failed to get db connection");
                                let _ = delivery.ack(BasicAckOptions::default()).await;
                                continue;
                            }
                        };

                        use diesel::prelude::*;
                        use crate::schema::credentials;

                        let is_permanent = data.sanction_type == "ban_permanent";
                        let _ = diesel::update(
                            credentials::table.filter(credentials::id.eq(data.user_id))
                        )
                        .set((
                            credentials::is_banned.eq(true),
                            credentials::ban_until.eq(if is_permanent { None } else { data.expires_at }),
                        ))
                        .execute(&mut conn);
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed to deserialize sanction.issued event");
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
