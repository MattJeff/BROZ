use lapin::{
    options::*, types::FieldTable, BasicProperties, Channel, Connection, ConnectionProperties,
    Consumer,
};
use serde::Serialize;

use crate::types::Event;

const EXCHANGE_NAME: &str = "broz.events";

#[derive(Clone)]
pub struct RabbitMQClient {
    channel: Channel,
}

impl RabbitMQClient {
    pub async fn connect(url: &str) -> Result<Self, lapin::Error> {
        let conn = Connection::connect(url, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await?;

        // Declare the topic exchange
        channel
            .exchange_declare(
                EXCHANGE_NAME,
                lapin::ExchangeKind::Topic,
                ExchangeDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        tracing::info!(url = %url, "connected to RabbitMQ");
        Ok(Self { channel })
    }

    /// Publish an event with a routing key
    pub async fn publish<T: Serialize>(
        &self,
        routing_key: &str,
        event: &Event<T>,
    ) -> Result<(), lapin::Error> {
        let payload = serde_json::to_vec(event)
            .map_err(|e| {
                tracing::error!(error = %e, "failed to serialize event");
                lapin::Error::IOError(std::sync::Arc::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e,
                )))
            })?;

        self.channel
            .basic_publish(
                EXCHANGE_NAME,
                routing_key,
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default()
                    .with_content_type("application/json".into())
                    .with_delivery_mode(2), // persistent
            )
            .await?
            .await?;

        tracing::debug!(
            routing_key = %routing_key,
            event_id = %event.id,
            "event published"
        );

        Ok(())
    }

    /// Declare a queue and bind it to routing keys
    pub async fn subscribe(
        &self,
        queue_name: &str,
        routing_keys: &[&str],
    ) -> Result<Consumer, lapin::Error> {
        // Declare durable queue
        self.channel
            .queue_declare(
                queue_name,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        // Bind queue to each routing key
        for key in routing_keys {
            self.channel
                .queue_bind(
                    queue_name,
                    EXCHANGE_NAME,
                    key,
                    QueueBindOptions::default(),
                    FieldTable::default(),
                )
                .await?;
        }

        // Start consuming
        let consumer = self.channel
            .basic_consume(
                queue_name,
                &format!("{queue_name}-consumer"),
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        tracing::info!(
            queue = %queue_name,
            bindings = ?routing_keys,
            "subscribed to RabbitMQ queue"
        );

        Ok(consumer)
    }

    pub fn channel(&self) -> &Channel {
        &self.channel
    }
}
