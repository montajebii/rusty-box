//! The Idea Bucket v2 — a zero-JavaScript, server-rendered rewrite of the
//! app in pure Rust.
//!
//! Run it with:
//!
//! ```text
//! IDEABUCKET_PORT=8088 IDEABUCKET_DATA=data/idea-bucket.json cargo run -p rustybox-server
//! ```
//!
//! Configuration comes from the environment (all optional):
//!
//! | variable           | default                  | meaning                    |
//! |--------------------|--------------------------|----------------------------|
//! | `IDEABUCKET_HOST`  | `127.0.0.1`              | bind address (local-only)  |
//! | `IDEABUCKET_PORT`  | `8088`                   | bind port                  |
//! | `IDEABUCKET_DATA`  | `data/idea-bucket.json`  | the JSON store file        |

use std::path::PathBuf;

use rustybox_core::Store;
use rustybox_server::routes;
use rustybox_server::state::AppState;

fn env_or(key: &str, default: &str) -> String {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => default.to_string(),
    }
}

fn main() {
    let host = env_or("IDEABUCKET_HOST", "127.0.0.1");
    let port: u16 = match env_or("IDEABUCKET_PORT", "8088").parse() {
        Ok(p) => p,
        Err(_) => {
            eprintln!("rustybox: IDEABUCKET_PORT must be a number, e.g. 8088");
            std::process::exit(1);
        }
    };
    let data = PathBuf::from(env_or("IDEABUCKET_DATA", "data/idea-bucket.json"));

    let store = match Store::load(&data) {
        Ok(store) => store,
        Err(e) => {
            eprintln!("rustybox: could not read {}: {e}", data.display());
            std::process::exit(1);
        }
    };
    let ideas = store.ideas.len();
    println!("rustybox: {} idea(s) from {}", ideas, data.display());
    println!("rustybox: serving on http://{host}:{port}");

    let state = std::sync::Arc::new(AppState::new(store));
    let app = routes::router(state);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    runtime.block_on(async move {
        let listener = match tokio::net::TcpListener::bind((host.as_str(), port)).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("rustybox: cannot bind {host}:{port}: {e}");
                std::process::exit(1);
            }
        };
        if let Err(e) = axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
        {
            eprintln!("rustybox: server error: {e}");
            std::process::exit(1);
        }
        println!("rustybox: bye");
    });
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    println!("rustybox: shutting down");
}
