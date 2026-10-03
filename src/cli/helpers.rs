use msucat::{MsuCatError, MsuClient, Result, UpdateSummary};

/// Check if a string is a standard 36-character UUID/GUID.
pub fn is_guid(s: &str) -> bool {
    let s = s.trim();
    if s.len() == 36 {
        let parts: Vec<&str> = s.split('-').collect();
        parts.len() == 5
            && parts[0].len() == 8
            && parts[1].len() == 4
            && parts[2].len() == 4
            && parts[3].len() == 4
            && parts[4].len() == 12
            && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
    } else {
        false
    }
}

/// Matches architecture queries with alias support (x64, arm64, x86).
pub fn matches_arch(text: &str, arch_query: &str) -> bool {
    let t = text.to_lowercase();
    let q = arch_query.to_lowercase();
    match q.as_str() {
        "x64" | "amd64" | "x86_64" => {
            t.contains("x64") || t.contains("amd64") || t.contains("x86_64") || t.contains("64-bit")
        }
        "arm64" | "aarch64" => t.contains("arm64") || t.contains("aarch64"),
        "x86" | "i386" | "i686" | "32-bit" | "32bit" => {
            if t.contains("x86_64") || t.contains("x64") || t.contains("amd64") {
                false
            } else {
                t.contains("x86")
                    || t.contains("i386")
                    || t.contains("32-bit")
                    || t.contains("32bit")
            }
        }
        _ => t.contains(&q),
    }
}

/// Extracts KB article number (e.g. "kb5020009") from an update title.
pub fn extract_kb_number(title: &str) -> Option<String> {
    let t = title.to_lowercase();
    if let Some(pos) = t.find("kb") {
        let after = &t[pos..];
        let digits: String = after
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        if digits.len() >= 4 {
            return Some(digits);
        }
    }
    None
}

/// Resolves the latest un-superseded update among sorted candidates by checking catalog supersedence chains.
pub async fn resolve_latest<'a>(
    client: &MsuClient,
    candidates: &'a [UpdateSummary],
) -> Result<&'a UpdateSummary> {
    if candidates.is_empty() {
        return Err(MsuCatError::NotFound(
            "No candidate updates to resolve".to_string(),
        ));
    }
    if candidates.len() == 1 {
        return Ok(&candidates[0]);
    }

    // Inspect top candidates (up to 8) to check supersedence
    let check_count = candidates.len().min(8);
    let top_candidates = &candidates[..check_count];

    let mut tasks = Vec::new();
    for c in top_candidates {
        let client_ref = client;
        let id = c.id.clone();
        tasks.push(async move { client_ref.get_details(&id).await.ok() });
    }
    let all_details = futures_util::future::join_all(tasks).await;

    let mut is_superseded = vec![false; check_count];

    for i in 0..check_count {
        for j in 0..check_count {
            if i == j {
                continue;
            }
            let c_i = &top_candidates[i];
            let c_j = &top_candidates[j];

            let kb_i = extract_kb_number(&c_i.title);
            let kb_j = extract_kb_number(&c_j.title);

            // Check if details of candidate i states it is superseded by candidate j
            if let Some(ref d_i) = all_details[i] {
                for s in &d_i.superseded_by {
                    let s_lower = s.to_lowercase();
                    if s_lower.contains(&c_j.title.to_lowercase())
                        || (kb_j.is_some() && s_lower.contains(kb_j.as_ref().unwrap()))
                    {
                        is_superseded[i] = true;
                        break;
                    }
                }
            }

            // Check if details of candidate j states it supersedes candidate i
            if !is_superseded[i] {
                if let Some(ref d_j) = all_details[j] {
                    for s in &d_j.supersedes {
                        let s_lower = s.to_lowercase();
                        if s_lower.contains(&c_i.title.to_lowercase())
                            || (kb_i.is_some() && s_lower.contains(kb_i.as_ref().unwrap()))
                        {
                            is_superseded[i] = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    // Return the first un-superseded candidate (which is the newest by date among un-superseded)
    for (idx, superseded) in is_superseded.iter().enumerate() {
        if !*superseded {
            return Ok(&top_candidates[idx]);
        }
    }

    // Fallback: newest date candidate
    Ok(&candidates[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_arch_aliases() {
        // x64 aliases
        assert!(matches_arch(
            "Update for Windows Server 2012 for x64-based Systems (KB5020009)",
            "x64"
        ));
        assert!(matches_arch(
            "Update for Windows Server 2012 for x64-based Systems (KB5020009)",
            "amd64"
        ));
        assert!(matches_arch(
            "Update for Windows Server 2012 for x64-based Systems (KB5020009)",
            "x86_64"
        ));
        assert!(matches_arch("Windows 10 AMD64 Architecture Update", "x64"));

        // arm64 aliases
        assert!(matches_arch("Windows 11 for ARM64-based Systems", "arm64"));
        assert!(matches_arch(
            "Windows 11 for ARM64-based Systems",
            "aarch64"
        ));
        assert!(!matches_arch("Windows 11 for ARM64-based Systems", "x64"));

        // x86 aliases
        assert!(matches_arch(
            "Update for Windows 7 for x86-based Systems",
            "x86"
        ));
        assert!(matches_arch(
            "Update for Windows 7 for x86-based Systems",
            "i386"
        ));
        assert!(matches_arch(
            "Update for Windows 7 for 32-bit Systems",
            "x86"
        ));
        // x86 query should not match x86_64
        assert!(!matches_arch("Update for x86_64 systems", "x86"));
    }

    #[test]
    fn test_extract_kb_number() {
        assert_eq!(
            extract_kb_number(
                "2022-11 Security Monthly Quality Rollup for Windows Server 2012 for x64-based Systems (KB5020009)"
            ),
            Some("kb5020009".to_string())
        );
        assert_eq!(extract_kb_number("Windows Update without KB"), None);
    }
}
