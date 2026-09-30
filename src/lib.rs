//! # msucat
//!
//! `msucat` is a fast, cross-platform client and parser for the **Microsoft Update Catalog**
//! (`catalog.update.microsoft.com`).
//!
//! Features:
//! - Search updates by KB number, title, architecture, classification
//! - Retrieve full update details (descriptions, supported products, MSRC severity, supersedence)
//! - Extract direct CDN download URLs (`.msu`, `.cab`, `.exe`)
//! - Extract verified SHA-256 and SHA-1 hashes directly from catalog manifests
//! - Resumable, streamed file downloads with on-the-fly checksum verification
//! - Works seamlessly on Windows, Linux, and macOS without COM APIs or Windows Update Agent

pub mod client;
pub mod error;
pub mod models;
pub mod parser;

pub use client::MsuClient;
pub use error::{MsuCatError, Result};
pub use models::{DownloadFile, UpdateDetails, UpdateSummary};
