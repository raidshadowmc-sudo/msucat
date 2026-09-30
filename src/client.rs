use crate::error::{MsuCatError, Result};
use crate::models::{DownloadFile, UpdateDetails, UpdateSummary};
use crate::parser::{parse_download_dialog, parse_search_results, parse_update_details};
use futures_util::StreamExt;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;

const CATALOG_BASE_URL: &str = "https://www.catalog.update.microsoft.com";
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// Client for interacting with the Microsoft Update Catalog.
#[derive(Clone, Debug)]
pub struct MsuClient {
    client: reqwest::Client,
    base_url: String,
}

impl Default for MsuClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MsuClient {
    /// Create a new `MsuClient` with default configuration.
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");

        Self {
            client,
            base_url: CATALOG_BASE_URL.to_string(),
        }
    }

    /// Search the catalog for updates matching `query`.
    /// Automatically handles pagination up to `max_pages` (if None, returns first page).
    pub async fn search(&self, query: &str) -> Result<Vec<UpdateSummary>> {
        self.search_with_limit(query, 1).await
    }

    /// Search the catalog with a maximum page limit.
    pub async fn search_with_limit(
        &self,
        query: &str,
        max_pages: usize,
    ) -> Result<Vec<UpdateSummary>> {
        let mut all_results = Vec::new();
        let mut current_page = 0;

        loop {
            let (results, has_next) = self.search_page(query, current_page).await?;
            if results.is_empty() {
                break;
            }

            all_results.extend(results);
            current_page += 1;

            if !has_next || current_page >= max_pages {
                break;
            }
        }

        Ok(all_results)
    }

    /// Fetch a single page of search results.
    pub async fn search_page(
        &self,
        query: &str,
        page_index: usize,
    ) -> Result<(Vec<UpdateSummary>, bool)> {
        let url = format!("{}/Search.aspx", self.base_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("q", query), ("page", &page_index.to_string())])
            .send()
            .await?;

        let html = resp.text().await?;
        Ok(parse_search_results(&html))
    }

    /// Get detailed metadata for an update by its GUID.
    pub async fn get_details(&self, update_id: &str) -> Result<UpdateDetails> {
        let url = format!("{}/ScopedViewInline.aspx", self.base_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("updateid", update_id)])
            .send()
            .await?;

        let html = resp.text().await?;
        if html.contains("<title>\r\n\tError\r\n</title>")
            || html.contains("<title>Error</title>")
            || html.contains("action=\"./ErrorInline.aspx\"")
            || html.contains("We did not find")
        {
            return Err(MsuCatError::NotFound(update_id.to_string()));
        }

        parse_update_details(&html, update_id)
    }

    /// Get direct file download URLs and hashes for a single update ID.
    pub async fn get_download_files(&self, update_id: &str) -> Result<Vec<DownloadFile>> {
        self.get_download_files_batch(&[update_id]).await
    }

    /// Get direct file download URLs and hashes for a batch of update IDs.
    pub async fn get_download_files_batch(&self, update_ids: &[&str]) -> Result<Vec<DownloadFile>> {
        if update_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut update_items = Vec::new();
        for &id in update_ids {
            update_items.push(json!({
                "size": 0,
                "languages": "",
                "uidInfo": id,
                "updateID": id,
            }));
        }

        let json_payload = serde_json::to_string(&update_items)?;
        let url = format!("{}/DownloadDialog.aspx", self.base_url);
        let referer_url = format!("{}/Search.aspx", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header(REFERER, &referer_url)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&[("updateIDs", &json_payload)])
            .send()
            .await?;

        let html = resp.text().await?;
        parse_download_dialog(&html)
    }

    /// Download a file with progress reporting and SHA-256 verification.
    ///
    /// * `url`: Direct CDN download URL
    /// * `destination`: Target path for the downloaded file
    /// * `expected_sha256_hex`: Optional SHA-256 hex string to verify
    /// * `on_progress`: Callback `(downloaded_bytes, total_bytes)`
    pub async fn download_file<P, F>(
        &self,
        url: &str,
        destination: P,
        expected_sha256_hex: Option<&str>,
        mut on_progress: F,
    ) -> Result<PathBuf>
    where
        P: AsRef<Path>,
        F: FnMut(u64, Option<u64>) + Send + 'static,
    {
        let dest_path = destination.as_ref().to_path_buf();

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // If file already exists and matches expected hash, skip downloading
        if dest_path.exists() {
            if let Some(expected) = expected_sha256_hex {
                if let Ok(mut existing_file) = File::open(&dest_path).await {
                    let mut hasher = Sha256::new();
                    let mut buf = [0u8; 65536];
                    use tokio::io::AsyncReadExt;
                    while let Ok(n) = existing_file.read(&mut buf).await {
                        if n == 0 {
                            break;
                        }
                        hasher.update(&buf[..n]);
                    }
                    let calculated = hex::encode(hasher.finalize());
                    if calculated.eq_ignore_ascii_case(expected) {
                        return Ok(dest_path);
                    }
                }
            }
        }

        let resp = self.client.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(MsuCatError::Download {
                file: dest_path.display().to_string(),
                reason: format!("HTTP {}", resp.status()),
            });
        }

        let total_size = resp.content_length();
        let part_path = dest_path.with_extension("part");

        let mut file = File::create(&part_path).await?;
        let mut stream = resp.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut hasher = Sha256::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
            hasher.update(&chunk);
            downloaded += chunk.len() as u64;
            on_progress(downloaded, total_size);
        }

        file.flush().await?;
        drop(file);

        let calculated_sha256 = hex::encode(hasher.finalize());

        // Verify checksum if provided
        if let Some(expected) = expected_sha256_hex {
            if !calculated_sha256.eq_ignore_ascii_case(expected) {
                let _ = fs::remove_file(&part_path).await;
                return Err(MsuCatError::ChecksumMismatch {
                    file: dest_path.display().to_string(),
                    expected: expected.to_string(),
                    actual: calculated_sha256,
                });
            }
        }

        // Rename .part to target path
        fs::rename(&part_path, &dest_path).await?;

        Ok(dest_path)
    }
}
