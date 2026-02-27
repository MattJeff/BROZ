pub mod config;
pub mod routes;

pub struct AppState {
    pub config: config::AppConfig,
    pub http_client: reqwest::Client,
    pub redis: tokio::sync::Mutex<redis::aio::ConnectionManager>,
    pub metrics_handle: metrics_exporter_prometheus::PrometheusHandle,
}
