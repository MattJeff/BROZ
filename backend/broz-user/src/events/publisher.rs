use uuid::Uuid;

use broz_shared::clients::rabbitmq::RabbitMQClient;
use broz_shared::types::event::{routing_keys, payloads, Event};

pub async fn publish_profile_updated(rabbitmq: &RabbitMQClient, profile_id: Uuid, credential_id: Uuid) {
    let event = Event::new(
        "broz-user",
        routing_keys::USER_PROFILE_UPDATED,
        payloads::ProfileUpdated {
            profile_id,
            credential_id,
        },
    )
    .with_user(credential_id);

    if let Err(e) = rabbitmq.publish(routing_keys::USER_PROFILE_UPDATED, &event).await {
        tracing::error!(error = %e, "failed to publish profile.updated event");
    }
}

pub async fn publish_onboarding_completed(rabbitmq: &RabbitMQClient, credential_id: Uuid, display_name: &str) {
    let event = Event::new(
        "broz-user",
        routing_keys::USER_ONBOARDING_COMPLETED,
        payloads::OnboardingCompleted {
            credential_id,
            display_name: display_name.to_string(),
        },
    )
    .with_user(credential_id);

    if let Err(e) = rabbitmq.publish(routing_keys::USER_ONBOARDING_COMPLETED, &event).await {
        tracing::error!(error = %e, "failed to publish onboarding.completed event");
    }
}

pub async fn publish_follow_requested(
    rabbitmq: &RabbitMQClient,
    follower_id: Uuid,
    following_id: Uuid,
    follower_display_name: &str,
) {
    let event = Event::new(
        "broz-user",
        routing_keys::USER_FOLLOW_REQUESTED,
        payloads::FollowRequested {
            follower_id,
            following_id,
            follower_display_name: follower_display_name.to_string(),
        },
    )
    .with_user(follower_id);

    if let Err(e) = rabbitmq.publish(routing_keys::USER_FOLLOW_REQUESTED, &event).await {
        tracing::error!(error = %e, "failed to publish follow.requested event");
    }
}

pub async fn publish_follow_accepted(rabbitmq: &RabbitMQClient, follower_id: Uuid, following_id: Uuid) {
    let event = Event::new(
        "broz-user",
        routing_keys::USER_FOLLOW_ACCEPTED,
        payloads::FollowAccepted {
            follower_id,
            following_id,
        },
    )
    .with_user(following_id);

    if let Err(e) = rabbitmq.publish(routing_keys::USER_FOLLOW_ACCEPTED, &event).await {
        tracing::error!(error = %e, "failed to publish follow.accepted event");
    }
}

pub async fn publish_follow_removed(rabbitmq: &RabbitMQClient, follower_id: Uuid, following_id: Uuid) {
    let event = Event::new(
        "broz-user",
        routing_keys::USER_FOLLOW_REMOVED,
        payloads::FollowRemoved {
            follower_id,
            following_id,
        },
    )
    .with_user(follower_id);

    if let Err(e) = rabbitmq.publish(routing_keys::USER_FOLLOW_REMOVED, &event).await {
        tracing::error!(error = %e, "failed to publish follow.removed event");
    }
}

pub async fn publish_like_sent(
    rabbitmq: &RabbitMQClient,
    liker_id: Uuid,
    liked_id: Uuid,
    liker_display_name: &str,
    match_session_id: Option<Uuid>,
) {
    let event = Event::new(
        "broz-user",
        routing_keys::USER_LIKE_SENT,
        payloads::LikeSent {
            liker_id,
            liked_id,
            liker_display_name: liker_display_name.to_string(),
            match_session_id,
        },
    )
    .with_user(liker_id);

    if let Err(e) = rabbitmq.publish(routing_keys::USER_LIKE_SENT, &event).await {
        tracing::error!(error = %e, "failed to publish like.sent event");
    }
}
