use uuid::Uuid;

use broz_shared::clients::rabbitmq::RabbitMQClient;
use broz_shared::types::event::{routing_keys, payloads, Event};

pub async fn publish_message_sent(
    rabbitmq: &RabbitMQClient,
    message_id: Uuid,
    conversation_id: Uuid,
    sender_id: Uuid,
    sender_display_name: &str,
    content_preview: &str,
) {
    let event = Event::new(
        "broz-messaging",
        routing_keys::MESSAGING_MESSAGE_SENT,
        payloads::MessageSent {
            message_id,
            conversation_id,
            sender_id,
            sender_display_name: sender_display_name.to_string(),
            content_preview: content_preview.to_string(),
        },
    )
    .with_user(sender_id);

    if let Err(e) = rabbitmq.publish(routing_keys::MESSAGING_MESSAGE_SENT, &event).await {
        tracing::error!(error = %e, "failed to publish message.sent event");
    }
}
