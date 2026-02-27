use uuid::Uuid;

use broz_shared::clients::rabbitmq::RabbitMQClient;
use broz_shared::types::event::{routing_keys, payloads, Event};

pub async fn publish_user_registered(rabbitmq: &RabbitMQClient, credential_id: Uuid, email: &str) {
    let event = Event::new(
        "broz-auth",
        routing_keys::AUTH_USER_REGISTERED,
        payloads::UserRegistered {
            credential_id,
            email: email.to_string(),
        },
    )
    .with_user(credential_id);

    if let Err(e) = rabbitmq.publish(routing_keys::AUTH_USER_REGISTERED, &event).await {
        tracing::error!(error = %e, "failed to publish user.registered event");
    }
}

pub async fn publish_user_banned(
    rabbitmq: &RabbitMQClient,
    credential_id: Uuid,
    is_banned: bool,
    ban_until: Option<chrono::DateTime<chrono::Utc>>,
) {
    let event = Event::new(
        "broz-auth",
        routing_keys::AUTH_USER_BANNED,
        payloads::UserBanned {
            credential_id,
            is_banned,
            ban_until,
        },
    )
    .with_user(credential_id);

    if let Err(e) = rabbitmq.publish(routing_keys::AUTH_USER_BANNED, &event).await {
        tracing::error!(error = %e, "failed to publish user.banned event");
    }
}
