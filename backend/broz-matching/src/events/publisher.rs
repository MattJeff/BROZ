use uuid::Uuid;

use broz_shared::clients::rabbitmq::RabbitMQClient;
use broz_shared::types::event::{payloads, routing_keys, Event};

pub async fn publish_session_started(
    rabbitmq: &RabbitMQClient,
    match_id: Uuid,
    user_a_id: Uuid,
    user_b_id: Uuid,
) {
    let event = Event::new(
        "broz-matching",
        routing_keys::MATCHING_SESSION_STARTED,
        payloads::MatchSessionStarted {
            match_id,
            user_a_id,
            user_b_id,
        },
    )
    .with_user(user_a_id);

    if let Err(e) = rabbitmq
        .publish(routing_keys::MATCHING_SESSION_STARTED, &event)
        .await
    {
        tracing::error!(error = %e, "failed to publish session.started event");
    }
}

pub async fn publish_session_ended(
    rabbitmq: &RabbitMQClient,
    match_id: Uuid,
    user_a_id: Uuid,
    user_b_id: Uuid,
    duration_secs: i32,
    end_reason: &str,
) {
    let event = Event::new(
        "broz-matching",
        routing_keys::MATCHING_SESSION_ENDED,
        payloads::MatchSessionEnded {
            match_id,
            user_a_id,
            user_b_id,
            duration_secs,
            end_reason: end_reason.to_string(),
        },
    )
    .with_user(user_a_id);

    if let Err(e) = rabbitmq
        .publish(routing_keys::MATCHING_SESSION_ENDED, &event)
        .await
    {
        tracing::error!(error = %e, "failed to publish session.ended event");
    }
}

pub async fn publish_livecam_requested(
    rabbitmq: &RabbitMQClient,
    request_id: Uuid,
    requester_id: Uuid,
    target_id: Uuid,
) {
    let event = Event::new(
        "broz-matching",
        routing_keys::MATCHING_LIVECAM_REQUESTED,
        payloads::LiveCamRequested {
            request_id,
            requester_id,
            target_id,
        },
    )
    .with_user(requester_id);

    if let Err(e) = rabbitmq
        .publish(routing_keys::MATCHING_LIVECAM_REQUESTED, &event)
        .await
    {
        tracing::error!(error = %e, "failed to publish livecam.requested event");
    }
}

pub async fn publish_livecam_responded(
    rabbitmq: &RabbitMQClient,
    request_id: Uuid,
    requester_id: Uuid,
    target_id: Uuid,
    accepted: bool,
) {
    let event = Event::new(
        "broz-matching",
        routing_keys::MATCHING_LIVECAM_RESPONDED,
        payloads::LiveCamResponded {
            request_id,
            requester_id,
            target_id,
            accepted,
        },
    )
    .with_user(target_id);

    if let Err(e) = rabbitmq
        .publish(routing_keys::MATCHING_LIVECAM_RESPONDED, &event)
        .await
    {
        tracing::error!(error = %e, "failed to publish livecam.responded event");
    }
}
