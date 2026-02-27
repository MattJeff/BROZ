use std::sync::Arc;
use futures_lite::StreamExt;
use lapin::options::BasicAckOptions;
use diesel::prelude::*;

use crate::AppState;
use crate::models::NewAnalyticsEvent;
use crate::schema::analytics_events;

/// Listen to ALL broz events via wildcard binding "broz.#".
/// Each event is inserted into the analytics_events table with:
/// - event_type from the routing key
/// - properties from the full JSON payload
/// - user_id extracted from the event envelope if present
pub async fn listen_all_events(state: Arc<AppState>) -> anyhow::Result<()> {
    let consumer = state.rabbitmq.subscribe(
        "broz-analytics.all",
        &["broz.#"],
    ).await?;

    tracing::info!("analytics subscriber listening on broz.# (all events)");

    let mut consumer = consumer;
    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => {
                let routing_key = delivery.routing_key.to_string();

                // Parse the full event JSON to extract user_id and store properties
                let event_json: serde_json::Value = match serde_json::from_slice(&delivery.data) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!(error = %e, routing_key = %routing_key, "failed to parse event JSON");
                        let _ = delivery.ack(BasicAckOptions::default()).await;
                        continue;
                    }
                };

                // Extract user_id from the event envelope (if present)
                let user_id = event_json
                    .get("user_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());

                let new_event = NewAnalyticsEvent {
                    user_id,
                    event_type: routing_key.clone(),
                    properties: Some(event_json),
                };

                // Insert into database
                let mut conn = match state.db.get() {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!(error = %e, "failed to get db connection");
                        let _ = delivery.ack(BasicAckOptions::default()).await;
                        continue;
                    }
                };

                match diesel::insert_into(analytics_events::table)
                    .values(&new_event)
                    .execute(&mut conn)
                {
                    Ok(_) => {
                        tracing::debug!(
                            routing_key = %routing_key,
                            user_id = ?user_id,
                            "analytics event recorded"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            routing_key = %routing_key,
                            "failed to insert analytics event"
                        );
                    }
                }

                let _ = delivery.ack(BasicAckOptions::default()).await;
            }
            Err(e) => {
                tracing::error!(error = %e, "analytics consumer error");
            }
        }
    }

    Ok(())
}
