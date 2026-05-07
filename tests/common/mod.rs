//! Shared helpers for integration tests.
//!
//! `cargo test` compiles each `tests/*.rs` file as a separate binary; modules
//! under `tests/common/` are imported into those binaries via `mod common;`.

#![allow(dead_code)] // Not every test binary uses every helper.

pub mod test_embedder;
