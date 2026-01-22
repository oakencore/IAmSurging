//! Prometheus metrics for the API server

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// Global Prometheus handle for metrics export
static PROMETHEUS_HANDLE: OnceCell<PrometheusHandle> = OnceCell::new();

/// Active WebSocket connection counter
static ACTIVE_WS_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

/// Initialize the Prometheus metrics recorder
pub fn init_metrics() -> PrometheusHandle {
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    if PROMETHEUS_HANDLE.set(handle.clone()).is_err() {
        panic!("metrics already initialized");
    }

    // Initialize gauges
    gauge!("active_websocket_connections").set(0.0);

    handle
}

/// Get the Prometheus handle for rendering metrics
pub fn get_prometheus_handle() -> Option<&'static PrometheusHandle> {
    PROMETHEUS_HANDLE.get()
}

/// Middleware to record request metrics
pub async fn track_metrics(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    let response = next.run(request).await;

    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    // Record metrics
    counter!("http_requests_total", "method" => method.clone(), "path" => path.clone(), "status" => status.clone()).increment(1);
    histogram!("http_request_duration_seconds", "method" => method, "path" => path, "status" => status).record(duration);

    response
}

/// Increment active WebSocket connection count
pub fn ws_connection_opened() {
    let count = ACTIVE_WS_CONNECTIONS.fetch_add(1, Ordering::SeqCst) + 1;
    gauge!("active_websocket_connections").set(count as f64);
}

/// Decrement active WebSocket connection count
pub fn ws_connection_closed() {
    let count = ACTIVE_WS_CONNECTIONS.fetch_sub(1, Ordering::SeqCst) - 1;
    gauge!("active_websocket_connections").set(count.max(0) as f64);
}

/// Get current active WebSocket connection count
pub fn get_active_ws_connections() -> usize {
    ACTIVE_WS_CONNECTIONS.load(Ordering::SeqCst)
}

/// Reset WebSocket connection count (for testing)
#[cfg(test)]
pub fn reset_ws_connections() {
    ACTIVE_WS_CONNECTIONS.store(0, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_connection_counter_initial() {
        reset_ws_connections();
        assert_eq!(get_active_ws_connections(), 0);
    }

    #[test]
    fn test_ws_connection_opened_increments() {
        reset_ws_connections();
        ws_connection_opened();
        assert_eq!(get_active_ws_connections(), 1);
        ws_connection_opened();
        assert_eq!(get_active_ws_connections(), 2);
        reset_ws_connections();
    }

    #[test]
    fn test_ws_connection_closed_decrements() {
        reset_ws_connections();
        ws_connection_opened();
        ws_connection_opened();
        ws_connection_opened();
        assert_eq!(get_active_ws_connections(), 3);

        ws_connection_closed();
        assert_eq!(get_active_ws_connections(), 2);

        ws_connection_closed();
        assert_eq!(get_active_ws_connections(), 1);

        reset_ws_connections();
    }

    #[test]
    fn test_ws_connection_lifecycle() {
        reset_ws_connections();

        // Simulate connections opening and closing
        ws_connection_opened();
        ws_connection_opened();
        assert_eq!(get_active_ws_connections(), 2);

        ws_connection_closed();
        assert_eq!(get_active_ws_connections(), 1);

        ws_connection_opened();
        assert_eq!(get_active_ws_connections(), 2);

        ws_connection_closed();
        ws_connection_closed();
        assert_eq!(get_active_ws_connections(), 0);

        reset_ws_connections();
    }
}
