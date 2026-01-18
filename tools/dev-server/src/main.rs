//! Development Server for Orbital OS
//!
//! Serves static files with the required COOP/COEP headers for
//! SharedArrayBuffer and Web Workers.

use axum::{
    body::Body,
    http::{HeaderValue, Request, StatusCode},
    response::Response,
    routing::get_service,
    Router,
};
use std::net::SocketAddr;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    // Serve static files from the www directory
    let serve_dir = ServeDir::new("apps/orbital-web/www")
        .precompressed_gzip()
        .precompressed_br();

    let app = Router::new()
        .fallback_service(get_service(serve_dir).handle_error(|_| async {
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        }))
        .layer(axum::middleware::from_fn(add_security_headers));

    println!("╔═══════════════════════════════════════════════════╗");
    println!("║           Orbital OS Development Server           ║");
    println!("╠═══════════════════════════════════════════════════╣");
    println!("║  URL: http://localhost:{}                       ║", port);
    println!("║  Press Ctrl+C to stop                             ║");
    println!("╚═══════════════════════════════════════════════════╝");
    println!();

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Add security headers required for SharedArrayBuffer and cross-origin isolation
async fn add_security_headers(
    request: Request<Body>,
    next: axum::middleware::Next,
) -> Response<Body> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Required for SharedArrayBuffer (used by Web Workers)
    headers.insert(
        "Cross-Origin-Opener-Policy",
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        "Cross-Origin-Embedder-Policy",
        HeaderValue::from_static("require-corp"),
    );

    response
}
