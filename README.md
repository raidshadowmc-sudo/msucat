# msucat

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/msucat.svg)](https://crates.io/crates/msucat)

Fast, cross-platform CLI and Rust library for searching, inspecting, and downloading update packages from the **Microsoft Update Catalog** (`catalog.update.microsoft.com`).

No Windows COM APIs, no Windows Update Agent, and no browser required. Works natively on **Linux**, **macOS**, and **Windows**.

---

## Why msucat?

The official Microsoft Update Catalog website relies on legacy ASP.NET WebForms, embedded JavaScript arrays, popup windows (`ScopedViewInline.aspx`, `DownloadDialog.aspx`), and form postbacks. Downloading hotfixes, cumulative updates, or drivers automatically in CI/CD, Docker containers, Ansible playbooks, or Linux servers has traditionally been cumbersome.

`msucat` reverse-engineers the Catalog's underlying protocol into a clean, modern CLI and Rust library:
* **Direct CDN URLs**: Extracts direct HTTP/HTTPS links (`.msu`, `.cab`, `.exe`) from `download.windowsupdate.com`.
* **Verified Hashes**: Extracts official SHA-256 and SHA-1 checksums directly from catalog manifests.
* **On-the-fly Verification**: Automatically validates downloaded files against official SHA-256 hashes.
* **Machine Readable**: Full `--json` support for seamless automation and pipeline integration.

---

## Installation

### From Crates.io
```bash
cargo install msucat --locked
```

### From Source
```bash
git clone https://github.com/raidshadowmc-sudo/msucat.git
cd msucat
cargo install --path .
```

---

## CLI Usage

### 1. Search for Updates
Search by KB number, title, or keywords:
```bash
# Search by KB number
msucat search KB5034441

# Search with filters
msucat search "Windows 11" --arch x64 --classification "Security Updates"

# Retrieve machine-readable JSON
msucat search "Windows Server 2022" --pages 2 --json
```

### 2. View Update Metadata & Supersedence
Inspect detailed information, including replacement/superseded updates, MSRC severity, and support links:
```bash
msucat info 20ff3247-bd92-4683-9094-c298e9c6125f
```

Example output:
```text
══════════════════════════════════════════════════════════════════════
  Update for Windows (KB4078130)
══════════════════════════════════════════════════════════════════════
  Update ID     : 20ff3247-bd92-4683-9094-c298e9c6125f
  Classification: Critical Updates
  Architectures : AMD64, X86
  KB Numbers    : 4078130
  Products      : Windows 10, Windows 11, Windows Server 2016...
  Reboot Behavior: Never restarts
──────────────────────────────────────────────────────────────────────
  Description:
  Install this update to resolve issues in Windows...
══════════════════════════════════════════════════════════════════════
```

### 3. Extract Direct Download Links & Checksums
Get direct download links along with verified SHA-256 and SHA-1 hashes without downloading:
```bash
msucat links 20ff3247-bd92-4683-9094-c298e9c6125f
```

Output:
```text
1. kb4078130_b86f0bf2dc0866a0e117ed2d4a5302fab0493a7b.exe
   URL: https://catalog.s.download.windowsupdate.com/c/msdownload/update/software/crup/2018/01/kb4078130_b86f0bf2dc0866a0e117ed2d4a5302fab0493a7b.exe
   SHA-256: 0592e39c28b53559806be5eaa8e15346b2f14109e47bd7d55b8058ff16195154
   SHA-1: b86f0bf2dc0866a0e117ed2d4a5302fab0493a7b
```

### 4. Download Updates with Automatic Integrity Verification
Download `.msu` / `.cab` / `.exe` files with a progress bar and automatic SHA-256 verification:
```bash
# Download to default directory (./downloads)
msucat download 20ff3247-bd92-4683-9094-c298e9c6125f

# Custom destination and filename filter
msucat download 20ff3247-bd92-4683-9094-c298e9c6125f --output /tmp/updates --filter "x64"
```

---

## Library Usage (Rust)

Add `msucat` to your `Cargo.toml`:
```toml
[dependencies]
msucat = "0.1.0"
```

### Example: Search and Download
```rust
use msucat::{MsuClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let client = MsuClient::new();

    // 1. Search for an update
    let updates = client.search("KB4078130").await?;
    if let Some(update) = updates.first() {
        println!("Found: {} ({})", update.title, update.id);

        // 2. Fetch direct download URLs and verified hashes
        let files = client.get_download_files(&update.id).await?;
        for file in files {
            println!("File: {} -> {}", file.file_name, file.url);

            // 3. Download with SHA-256 verification
            client.download_file(
                &file.url,
                format!("./downloads/{}", file.file_name),
                file.sha256_hex.as_deref(),
                |downloaded, total| {
                    if let Some(total) = total {
                        print!("\rProgress: {} / {} bytes", downloaded, total);
                    }
                },
            ).await?;
            println!("\nDownloaded and verified!");
        }
    }

    Ok(())
}
```

---

## License

Licensed under the [MIT License](LICENSE).
