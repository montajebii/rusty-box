//! # rustybox-server
//!
//! The zero-JavaScript web server for The Idea Bucket v2: axum routes,
//! askama templates, and the pure view math (glass level, blobs, dates).
//! Exposed as a library so the HTTP integration suite drives the real
//! router in-process — no sockets, no browser.

pub mod config;
pub mod flash;
pub mod form;
pub mod routes;
pub mod state;
pub mod view;
