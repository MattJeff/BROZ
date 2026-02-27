use chrono::{DateTime, Utc};
use uuid::Uuid;

use broz_shared::clients::rabbitmq::RabbitMQClient;
use broz_shared::types::event::{routing_keys, payloads, Event};

pub async fn publish_report_created(
    rabbitmq: &RabbitMQClient,
    report_id: Uuid,
    reporter_id: Uuid,
    reported_id: Uuid,
    report_type: &str,
) {
    let event = Event::new(
        "broz-moderation",
        routing_keys::MODERATION_REPORT_CREATED,
        payloads::ReportCreated {
            report_id,
            reporter_id,
            reported_id,
            report_type: report_type.to_string(),
        },
    )
    .with_user(reporter_id);

    if let Err(e) = rabbitmq.publish(routing_keys::MODERATION_REPORT_CREATED, &event).await {
        tracing::error!(error = %e, "failed to publish report.created event");
    }
}

pub async fn publish_sanction_issued(
    rabbitmq: &RabbitMQClient,
    sanction_id: Uuid,
    user_id: Uuid,
    sanction_type: &str,
    reason: &str,
    expires_at: Option<DateTime<Utc>>,
) {
    let event = Event::new(
        "broz-moderation",
        routing_keys::MODERATION_SANCTION_ISSUED,
        payloads::SanctionIssued {
            sanction_id,
            user_id,
            sanction_type: sanction_type.to_string(),
            reason: reason.to_string(),
            expires_at,
        },
    )
    .with_user(user_id);

    if let Err(e) = rabbitmq.publish(routing_keys::MODERATION_SANCTION_ISSUED, &event).await {
        tracing::error!(error = %e, "failed to publish sanction.issued event");
    }
}

pub async fn publish_sanction_lifted(
    rabbitmq: &RabbitMQClient,
    sanction_id: Uuid,
    user_id: Uuid,
) {
    let event = Event::new(
        "broz-moderation",
        routing_keys::MODERATION_SANCTION_LIFTED,
        payloads::SanctionLifted {
            sanction_id,
            user_id,
        },
    )
    .with_user(user_id);

    if let Err(e) = rabbitmq.publish(routing_keys::MODERATION_SANCTION_LIFTED, &event).await {
        tracing::error!(error = %e, "failed to publish sanction.lifted event");
    }
}
