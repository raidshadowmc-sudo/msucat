use serde::{Deserialize, Serialize};

/// Summary of an update row from search results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateSummary {
    /// Update GUID (e.g. "20ff3247-bd92-4683-9094-c298e9c6125f")
    pub id: String,
    /// Title of the update
    pub title: String,
    /// Targeted products (e.g. "Windows 10, Windows 11")
    pub products: String,
    /// Classification (e.g. "Security Updates", "Critical Updates")
    pub classification: String,
    /// Last modified / release date string
    pub last_updated: String,
    /// Version string
    pub version: String,
    /// Human readable size string (e.g. "24 KB", "1.2 GB")
    pub size: String,
    /// Exact size in bytes
    pub size_bytes: u64,
}

/// Detailed metadata of a single update from ScopedView.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateDetails {
    pub id: String,
    pub title: String,
    pub description: String,
    pub architectures: Vec<String>,
    pub classification: String,
    pub supported_products: Vec<String>,
    pub supported_languages: Vec<String>,
    pub msrc_number: String,
    pub msrc_severity: String,
    pub kb_numbers: Vec<String>,
    pub more_info_urls: Vec<String>,
    pub support_urls: Vec<String>,
    pub restart_behavior: String,
    pub may_request_user_input: bool,
    pub must_be_installed_exclusively: bool,
    pub requires_network_connectivity: bool,
    pub uninstall_notes: String,
    pub supersedes: Vec<String>,
    pub superseded_by: Vec<String>,
}

/// Direct download file item extracted from DownloadDialog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadFile {
    pub update_id: String,
    pub title: String,
    pub file_name: String,
    pub url: String,
    pub sha256_base64: Option<String>,
    pub sha256_hex: Option<String>,
    pub sha1_base64: Option<String>,
    pub sha1_hex: Option<String>,
}
