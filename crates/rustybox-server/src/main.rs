//! Rusty Box — a zero-JavaScript, server-rendered home for your ideas.
//!
//! Run it with:
//!
//! ```text
//! RUSTYBOX_PORT=8088 RUSTYBOX_DATA=data/rusty-box.json cargo run -p rustybox-server
//! ```
//!
//! Configuration comes from the environment (all optional). The Rusty Box
//! names are primary; the pre-rename `IDEABUCKET_*` names are still honored
//! so existing setups keep working:
//!
//! | variable           | default                        | meaning                    |
//! |--------------------|--------------------------------|----------------------------|
//! | `RUSTYBOX_HOST`    | `127.0.0.1`                    | bind address (local-only)  |
//! | `RUSTYBOX_PORT`    | `8088`                         | bind port                  |
//! | `RUSTYBOX_DATA`    | `data/rusty-box.json`          | the JSON store file        |
//!
//! The boot log states exactly which name supplied each value.

use rustybox_core::Store;
use rustybox_server::config::Config;
use rustybox_server::routes;
use rustybox_server::state::AppState;

fn main() {
    let cfg = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("rustybox: {e}");
            std::process::exit(1);
        }
    };

    let (store, quarantine) = match Store::load_reported(&cfg.data) {
        Ok(found) => found,
        Err(e) => {
            eprintln!("rustybox: could not read {}: {e}", cfg.data.display());
            std::process::exit(1);
        }
    };
    if let Some(backup) = &quarantine {
        println!(
            "rustybox: the data file was corrupt and was moved to {}",
            backup.display()
        );
    }
    let ideas = store.ideas.len();
    println!("rustybox: {} idea(s) from {}", ideas, cfg.data.display());
    println!("rustybox: {}", cfg.host_line());
    println!("rustybox: {}", cfg.port_line());
    println!("rustybox: {}", cfg.data_line());

    let state = std::sync::Arc::new(AppState::with_quarantine(store, quarantine));
    let app = routes::router(state);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let bind_host = cfg.host.clone();
    let bind_port = cfg.port;
    runtime.block_on(async move {
        let listener = match tokio::net::TcpListener::bind((bind_host.as_str(), bind_port)).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("rustybox: cannot bind {bind_host}:{bind_port}: {e}");
                std::process::exit(1);
            }
        };
        // only now is "serving on" true — a bind conflict must never print it
        println!("rustybox: serving on http://{}:{}", bind_host, bind_port);
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
