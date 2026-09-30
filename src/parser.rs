use crate::error::{MsuCatError, Result};
use crate::models::{DownloadFile, UpdateDetails, UpdateSummary};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use regex::Regex;
use scraper::{Html, Selector};
use std::sync::LazyLock;

static RE_GUID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")
        .expect("valid regex")
});

static RE_DOWNLOAD_INFO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"downloadInformation\[(\d+)\]\.(updateID|enTitle)\s*=\s*'([^']*)';"#)
        .expect("valid regex")
});

static RE_FILE_INFO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"downloadInformation\[(\d+)\]\.files\[(\d+)\]\.(url|digest|sha256|fileName)\s*=\s*'([^']*)';"#)
        .expect("valid regex")
});

/// Parse search results table from Search.aspx HTML.
pub fn parse_search_results(html: &str) -> (Vec<UpdateSummary>, bool) {
    let document = Html::parse_document(html);
    let mut results = Vec::new();

    // Selector for rows in the results table that have an ID containing a GUID
    let tr_selector = match Selector::parse("tr") {
        Ok(s) => s,
        Err(_) => return (results, false),
    };

    let a_selector = Selector::parse("a").ok();
    let span_selector = Selector::parse("span").ok();
    let td_selector = Selector::parse("td").ok();

    for tr in document.select(&tr_selector) {
        let row_id = match tr.value().attr("id") {
            Some(id) if id.contains('_') => id,
            _ => continue,
        };

        // Extract GUID from row ID
        let guid = match RE_GUID.find(row_id) {
            Some(m) => m.as_str().to_string(),
            None => continue,
        };

        // Collect all td elements in row
        let tds: Vec<_> = if let Some(ref sel) = td_selector {
            tr.select(sel).collect()
        } else {
            Vec::new()
        };

        if tds.len() < 7 {
            continue;
        }

        // C1: Title link
        let title = if let Some(ref sel) = a_selector {
            tds[1]
                .select(sel)
                .next()
                .map(|a| a.text().collect::<String>().trim().to_string())
                .unwrap_or_else(|| tds[1].text().collect::<String>().trim().to_string())
        } else {
            tds[1].text().collect::<String>().trim().to_string()
        };

        if title.is_empty() {
            continue;
        }

        // C2: Products
        let products = tds[2].text().collect::<String>().trim().to_string();

        // C3: Classification
        let classification = tds[3].text().collect::<String>().trim().to_string();

        // C4: Last Updated
        let last_updated = tds[4].text().collect::<String>().trim().to_string();

        // C5: Version
        let version = tds[5].text().collect::<String>().trim().to_string();

        // C6: Size and size in bytes
        let mut size = "0 B".to_string();
        let mut size_bytes = 0u64;

        if let Some(ref sel) = span_selector {
            for span in tds[6].select(sel) {
                if let Some(span_id) = span.value().attr("id") {
                    if span_id.ends_with("_originalSize") {
                        let text = span.text().collect::<String>().trim().to_string();
                        size_bytes = text.parse::<u64>().unwrap_or(0);
                    } else if span_id.ends_with("_size") {
                        size = span.text().collect::<String>().trim().to_string();
                    }
                }
            }
        }

        if size.is_empty() || size == "0 B" {
            let direct_text = tds[6].text().collect::<String>().trim().to_string();
            if !direct_text.is_empty() {
                size = direct_text;
            }
        }

        results.push(UpdateSummary {
            id: guid,
            title,
            products,
            classification,
            last_updated,
            version,
            size,
            size_bytes,
        });
    }

    // Check if there are more pages
    let has_next_page = html.contains("data-commandName=\"next\"")
        || html.contains("data-commandname=\"next\"")
        || html.contains("goToNextPage");

    (results, has_next_page)
}

/// Parse metadata from ScopedViewInline.aspx HTML.
pub fn parse_update_details(html: &str, update_id: &str) -> Result<UpdateDetails> {
    let document = Html::parse_document(html);
    let mut details = UpdateDetails {
        id: update_id.to_string(),
        ..Default::default()
    };

    let select_text = |selector_str: &str| -> String {
        if let Ok(sel) = Selector::parse(selector_str) {
            if let Some(el) = document.select(&sel).next() {
                return el.text().collect::<String>().trim().to_string();
            }
        }
        String::new()
    };

    let select_links = |selector_str: &str| -> Vec<String> {
        let mut links = Vec::new();
        if let Ok(sel) = Selector::parse(selector_str) {
            for el in document.select(&sel) {
                if let Some(href) = el.value().attr("href") {
                    let h = href.trim().to_string();
                    if !h.is_empty() && !links.contains(&h) {
                        links.push(h);
                    }
                }
            }
        }
        links
    };

    details.title = select_text("#ScopedViewHandler_titleText");
    if details.title.is_empty() {
        details.title = select_text(".labelTitle");
    }

    details.description = select_text("#ScopedViewHandler_desc");
    details.classification = select_text("#ScopedViewHandler_labelClassification_Separator + span");
    if details.classification.is_empty() {
        details.classification = select_text("#classificationDiv");
        if let Some(stripped) = details.classification.strip_prefix("Classification:") {
            details.classification = stripped.trim().to_string();
        }
    }

    // Architecture
    let arch_raw = select_text("#archDiv");
    if let Some(stripped) = arch_raw.strip_prefix("Architecture:") {
        for a in stripped.split(',') {
            let cleaned = a.trim().to_string();
            if !cleaned.is_empty() {
                details.architectures.push(cleaned);
            }
        }
    }

    // Supported Products
    let prod_raw = select_text("#productsDiv");
    if let Some(stripped) = prod_raw.strip_prefix("Supported products:") {
        for p in stripped.split(',') {
            let cleaned = p.trim().to_string();
            if !cleaned.is_empty() {
                details.supported_products.push(cleaned);
            }
        }
    }

    // Supported Languages
    let lang_raw = select_text("#languagesDiv");
    if let Some(stripped) = lang_raw.strip_prefix("Supported languages:") {
        for l in stripped.split(',') {
            let cleaned = l.trim().to_string();
            if !cleaned.is_empty() {
                details.supported_languages.push(cleaned);
            }
        }
    }

    details.msrc_number = select_text("#ScopedViewHandler_labelSecurityBulliten_Separator + span");
    if details.msrc_number.is_empty() {
        let msrc_raw = select_text("#securityBullitenDiv");
        if let Some(s) = msrc_raw.strip_prefix("MSRC Number:") {
            details.msrc_number = s.trim().to_string();
        }
    }

    details.msrc_severity = select_text("#ScopedViewHandler_msrcSeverity");

    // KB Article numbers
    let kb_raw = select_text("#kbDiv");
    if let Some(s) = kb_raw.strip_prefix("KB article numbers:") {
        for kb in s.split(',') {
            let cleaned = kb.trim().to_string();
            if !cleaned.is_empty() {
                details.kb_numbers.push(cleaned);
            }
        }
    }

    details.more_info_urls = select_links("#moreInfoDiv a");
    details.support_urls = select_links("#suportUrlDiv a");

    // Install Behavior
    details.restart_behavior = select_text("#ScopedViewHandler_rebootBehavior");
    let user_input = select_text("#ScopedViewHandler_userInput");
    details.may_request_user_input = user_input.eq_ignore_ascii_case("yes");

    let network = select_text("#ScopedViewHandler_connectivity");
    details.requires_network_connectivity = network.eq_ignore_ascii_case("yes");

    details.uninstall_notes = select_text("#uninstallNotesDiv");
    if let Some(s) = details.uninstall_notes.strip_prefix("Uninstall Notes:") {
        details.uninstall_notes = s.trim().to_string();
    }

    // Superseded / Supersedes
    let superseded_by_raw = select_text("#supersededbyInfo");
    if !superseded_by_raw.is_empty() && superseded_by_raw != "n/a" {
        for s in superseded_by_raw.split(',') {
            let c = s.trim().to_string();
            if !c.is_empty() {
                details.superseded_by.push(c);
            }
        }
    }

    let supersedes_raw = select_text("#supersedesInfo");
    if !supersedes_raw.is_empty() && supersedes_raw != "n/a" {
        for s in supersedes_raw.split(',') {
            let c = s.trim().to_string();
            if !c.is_empty() {
                details.supersedes.push(c);
            }
        }
    }

    Ok(details)
}

/// Parse direct file download URLs and hashes from DownloadDialog.aspx HTML.
pub fn parse_download_dialog(html: &str) -> Result<Vec<DownloadFile>> {
    use std::collections::HashMap;

    #[derive(Default)]
    struct RawUpdate {
        update_id: String,
        title: String,
        files: HashMap<usize, RawFile>,
    }

    #[derive(Default)]
    struct RawFile {
        url: String,
        digest: String,
        sha256: String,
        file_name: String,
    }

    let mut updates: HashMap<usize, RawUpdate> = HashMap::new();

    // Scan for update-level metadata
    for cap in RE_DOWNLOAD_INFO.captures_iter(html) {
        let u_idx: usize = cap[1].parse().unwrap_or(0);
        let prop = &cap[2];
        let val = &cap[3];

        let entry = updates.entry(u_idx).or_default();
        match prop {
            "updateID" => entry.update_id = val.to_string(),
            "enTitle" => entry.title = val.to_string(),
            _ => {}
        }
    }

    // Scan for file-level metadata
    for cap in RE_FILE_INFO.captures_iter(html) {
        let u_idx: usize = cap[1].parse().unwrap_or(0);
        let f_idx: usize = cap[2].parse().unwrap_or(0);
        let prop = &cap[3];
        let val = &cap[4];

        let entry = updates.entry(u_idx).or_default();
        let file = entry.files.entry(f_idx).or_default();

        match prop {
            "url" => file.url = val.to_string(),
            "digest" => file.digest = val.to_string(),
            "sha256" => file.sha256 = val.to_string(),
            "fileName" => file.file_name = val.to_string(),
            _ => {}
        }
    }

    let mut files_result = Vec::new();

    for (_, u) in updates {
        for (_, f) in u.files {
            if f.url.is_empty() {
                continue;
            }

            // Derive file name from URL if missing
            let file_name = if !f.file_name.is_empty() {
                f.file_name
            } else if let Some(last_slash) = f.url.rfind('/') {
                f.url[last_slash + 1..].to_string()
            } else {
                "download.bin".to_string()
            };

            // Compute Hex SHA256 from Base64
            let sha256_base64 = if !f.sha256.is_empty() {
                Some(f.sha256.clone())
            } else {
                None
            };

            let sha256_hex = sha256_base64
                .as_ref()
                .and_then(|b64| BASE64.decode(b64).ok().map(hex::encode));

            // Compute Hex SHA1 from Base64
            let sha1_base64 = if !f.digest.is_empty() {
                Some(f.digest.clone())
            } else {
                None
            };

            let sha1_hex = sha1_base64
                .as_ref()
                .and_then(|b64| BASE64.decode(b64).ok().map(hex::encode));

            files_result.push(DownloadFile {
                update_id: u.update_id.clone(),
                title: u.title.clone(),
                file_name,
                url: f.url,
                sha256_base64,
                sha256_hex,
                sha1_base64,
                sha1_hex,
            });
        }
    }

    // Fallback: If no structured JavaScript array found, extract URLs with regex
    if files_result.is_empty() {
        let re_urls = Regex::new(r#"https?://[^\s'"<>]+\.(?:msu|cab|exe)"#).unwrap();
        for mat in re_urls.find_iter(html) {
            let url = mat.as_str().to_string();
            let file_name = if let Some(idx) = url.rfind('/') {
                url[idx + 1..].to_string()
            } else {
                "update.bin".to_string()
            };

            files_result.push(DownloadFile {
                update_id: String::new(),
                title: String::new(),
                file_name,
                url,
                sha256_base64: None,
                sha256_hex: None,
                sha1_base64: None,
                sha1_hex: None,
            });
        }
    }

    if files_result.is_empty() {
        return Err(MsuCatError::Parse(
            "No download links found in DownloadDialog response".to_string(),
        ));
    }

    Ok(files_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_download_dialog_js() {
        let sample = r#"
        var downloadInformation = new Array();
        downloadInformation[0] = new Object();
        downloadInformation[0].updateID ='20ff3247-bd92-4683-9094-c298e9c6125f';
        downloadInformation[0].enTitle ='Update for Windows (KB4078130) ';
        downloadInformation[0].files = new Array();
        downloadInformation[0].files[0] = new Object();
        downloadInformation[0].files[0].url = 'https://catalog.s.download.windowsupdate.com/c/msdownload/update/software/crup/2018/01/kb4078130_b86f0bf2dc0866a0e117ed2d4a5302fab0493a7b.exe';
        downloadInformation[0].files[0].digest = 'uG8L8twIZqDhF+0tSlMC+rBJOns=';
        downloadInformation[0].files[0].sha256 = 'BZLjnCi1NVmAa+XqqOFTRrLxQQnke9fVW4BY/xYZUVQ=';
        downloadInformation[0].files[0].fileName = 'kb4078130_b86f0bf2dc0866a0e117ed2d4a5302fab0493a7b.exe';
        "#;

        let files = parse_download_dialog(sample).expect("should parse files");
        assert_eq!(files.len(), 1);
        let f = &files[0];
        assert_eq!(f.update_id, "20ff3247-bd92-4683-9094-c298e9c6125f");
        assert_eq!(
            f.file_name,
            "kb4078130_b86f0bf2dc0866a0e117ed2d4a5302fab0493a7b.exe"
        );
        assert_eq!(
            f.url,
            "https://catalog.s.download.windowsupdate.com/c/msdownload/update/software/crup/2018/01/kb4078130_b86f0bf2dc0866a0e117ed2d4a5302fab0493a7b.exe"
        );
        assert_eq!(
            f.sha256_base64.as_deref(),
            Some("BZLjnCi1NVmAa+XqqOFTRrLxQQnke9fVW4BY/xYZUVQ=")
        );
        assert_eq!(
            f.sha1_base64.as_deref(),
            Some("uG8L8twIZqDhF+0tSlMC+rBJOns=")
        );
        assert!(f.sha256_hex.is_some());
        assert!(f.sha1_hex.is_some());
    }

    #[test]
    fn test_parse_search_results_sample() {
        let sample = r#"
        <table>
        <tr id="20ff3247-bd92-4683-9094-c298e9c6125f_R0">
            <td id="20ff3247-bd92-4683-9094-c298e9c6125f_C0_R0"></td>
            <td id="20ff3247-bd92-4683-9094-c298e9c6125f_C1_R0">
                <a id="20ff3247-bd92-4683-9094-c298e9c6125f_link" href="javascript:void(0)">Update for Windows (KB4078130)</a>
            </td>
            <td id="20ff3247-bd92-4683-9094-c298e9c6125f_C2_R0">Windows 10, Windows 11</td>
            <td id="20ff3247-bd92-4683-9094-c298e9c6125f_C3_R0">Critical Updates</td>
            <td id="20ff3247-bd92-4683-9094-c298e9c6125f_C4_R0">1/26/2018</td>
            <td id="20ff3247-bd92-4683-9094-c298e9c6125f_C5_R0">n/a</td>
            <td id="20ff3247-bd92-4683-9094-c298e9c6125f_C6_R0">
                <span id="20ff3247-bd92-4683-9094-c298e9c6125f_size">24 KB</span>
                <span class="noDisplay" id="20ff3247-bd92-4683-9094-c298e9c6125f_originalSize">25288</span>
            </td>
            <td id="20ff3247-bd92-4683-9094-c298e9c6125f_C7_R0">
                <input id="20ff3247-bd92-4683-9094-c298e9c6125f" type="button" value="Download" />
            </td>
        </tr>
        </table>
        "#;

        let (results, has_next) = parse_search_results(sample);
        assert_eq!(results.len(), 1);
        let u = &results[0];
        assert_eq!(u.id, "20ff3247-bd92-4683-9094-c298e9c6125f");
        assert_eq!(u.title, "Update for Windows (KB4078130)");
        assert_eq!(u.products, "Windows 10, Windows 11");
        assert_eq!(u.classification, "Critical Updates");
        assert_eq!(u.last_updated, "1/26/2018");
        assert_eq!(u.size, "24 KB");
        assert_eq!(u.size_bytes, 25288);
        assert!(!has_next);
    }
}
