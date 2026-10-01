//! # msucat
//!
//! `msucat` is a fast, cross-platform client and parser for the **Microsoft Update Catalog**
//! (`catalog.update.microsoft.com`).
//!
//! Features:
//! - Search updates by KB number, title, architecture, classification
//! - Real multi-page pagination support
//! - Retrieve full update details (descriptions, supported products, MSRC severity, supersedence)
//! - Extract direct CDN download URLs (`.msu`, `.cab`, `.exe`)
//! - Extract verified SHA-256 and SHA-1 hashes directly from catalog manifests
//! - Safe streaming file downloads with temporary `.part` files, on-the-fly checksum verification, and existing-file integrity checks
//! - Works seamlessly on Windows, Linux, and macOS without COM APIs or Windows Update Agent
//!
//! *Disclaimer: `msucat` is an unofficial open-source scraper and client for Microsoft's legacy Update Catalog WebForms interface. It is not affiliated with or endorsed by Microsoft and comes with no SLA.*

pub mod client;
pub mod error;
pub mod models;
pub mod parser;

pub use client::MsuClient;
pub use error::{MsuCatError, Result};
pub use models::{DownloadFile, UpdateDetails, UpdateSummary};
pub use parser::parse_catalog_date;
