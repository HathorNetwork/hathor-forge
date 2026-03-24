//! Transparent CORS reverse proxy for the fullnode API.
//!
//! hathor-core does not handle browser CORS preflight (OPTIONS) requests.
//! This proxy listens on the public fullnode port and forwards all requests
//! to the internal fullnode port, adding the required CORS headers so browser
//! dapps can access the API directly.

use axum::{
    body::Body,
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use tokio::sync::oneshot;
use tracing::{error, info};

/// Start the CORS proxy.
///
/// Listens on `public_port` and forwards all requests to `internal_port`.
/// Returns a shutdown sender — drop or send `()` to stop the proxy.
pub fn start(
    public_port: u16,
    internal_port: u16,
) -> oneshot::Sender<()> {
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let upstream_base = format!("http://127.0.0.1:{}", internal_port);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .pool_max_idle_per_host(10)
        .build()
        .expect("Failed to build CORS proxy HTTP client");

    let app = Router::new()
        .fallback(any(move |req: Request| {
            let upstream_base = upstream_base.clone();
            let client = client.clone();
            async move { proxy_request(req, &upstream_base, &client).await }
        }));

    tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(format!("0.0.0.0:{}", public_port))
            .await
        {
            Ok(l) => l,
            Err(e) => {
                error!(port = public_port, %e, "CORS proxy failed to bind");
                return;
            }
        };
        info!(
            public_port,
            internal_port, "CORS proxy listening"
        );

        axum::serve(listener, app)
            .with_graceful_shutdown(async { let _ = shutdown_rx.await; })
            .await
            .ok();
    });

    shutdown_tx
}

async fn proxy_request(
    req: Request,
    upstream_base: &str,
    client: &reqwest::Client,
) -> Response<Body> {
    // Handle preflight OPTIONS requests directly
    if req.method() == axum::http::Method::OPTIONS {
        return cors_response(StatusCode::NO_CONTENT, Body::empty(), None);
    }

    // Build upstream URL
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let target_url = format!("{}{}", upstream_base, path_and_query);

    let method = req.method().clone();
    let content_type = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Read request body
    let body_bytes = match axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return cors_response(
                StatusCode::BAD_REQUEST,
                Body::from("Failed to read request body"),
                None,
            );
        }
    };

    // Forward to upstream
    let mut builder = client.request(
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap(),
        &target_url,
    );
    if let Some(ct) = content_type {
        builder = builder.header("content-type", ct);
    }
    if !body_bytes.is_empty() {
        builder = builder.body(body_bytes.to_vec());
    }

    match builder.send().await {
        Ok(resp) => {
            let status =
                StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::OK);
            let ct = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let resp_bytes = resp.bytes().await.unwrap_or_default();
            cors_response(status, Body::from(resp_bytes), ct.as_deref())
        }
        Err(e) => cors_response(
            StatusCode::BAD_GATEWAY,
            Body::from(
                serde_json::json!({"success": false, "error": e.to_string()}).to_string(),
            ),
            Some("application/json"),
        ),
    }
}

fn cors_response(status: StatusCode, body: Body, content_type: Option<&str>) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            "GET, POST, PUT, DELETE, OPTIONS",
        )
        .header(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            "content-type, x-prototype-version, x-requested-with",
        )
        .header(header::ACCESS_CONTROL_MAX_AGE, "604800")
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(content_type.unwrap_or("application/json"))
                .unwrap_or_else(|_| HeaderValue::from_static("application/json")),
        )
        .body(body)
        .unwrap()
}
