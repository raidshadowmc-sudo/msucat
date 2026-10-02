# msucat

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/msucat.svg)](https://crates.io/crates/msucat)

Fast, cross-platform CLI and Rust library for searching, inspecting, and downloading update packages from the **Microsoft Update Catalog** (`catalog.update.microsoft.com`).

No Windows COM APIs, no Windows Update Agent, and no browser required. Works natively on **Linux**, **macOS**, and **Windows**.

<p align="center">
  <img src="assets/demo.gif" alt="msucat in action" width="800">
</p>

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

### Prebuilt Binaries (Linux, macOS, Windows)

Standalone pre-compiled binaries are published for every release on [GitHub Releases](https://github.com/raidshadowmc-sudo/msucat/releases/latest):

| Target Platform | Architecture | Binary Package |
|---|---|---|
| **Linux** | `x86_64` (glibc 2.17+) | [`msucat-*-x86_64-unknown-linux-gnu.tar.gz`](https://github.com/raidshadowmc-sudo/msucat/releases/latest) |
| **Linux** | `aarch64` (ARM64) | [`msucat-*-aarch64-unknown-linux-gnu.tar.gz`](https://github.com/raidshadowmc-sudo/msucat/releases/latest) |
| **Windows** | `x86_64` (MSVC) | [`msucat-*-x86_64-pc-windows-msvc.zip`](https://github.com/raidshadowmc-sudo/msucat/releases/latest) |
| **macOS** | Apple Silicon (`aarch64`) | [`msucat-*-aarch64-apple-darwin.tar.gz`](https://github.com/raidshadowmc-sudo/msucat/releases/latest) |
| **macOS** | Intel (`x86_64`) | [`msucat-*-x86_64-apple-darwin.tar.gz`](https://github.com/raidshadowmc-sudo/msucat/releases/latest) |

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

### 🚀 1. The One-Shot Downloader (`get`)
Search, filter, resolve CDN links, stream download, and verify official SHA-256 hashes in a single command — no browser, no manual copying of GUIDs:

```bash
# Download a specific KB directly
msucat get KB5034441

# Download latest security rollup for Windows Server 2022
msucat get "Windows Server 2022" --arch x64 --class security --latest

# Inspect direct CDN URLs and official SHA-256 hashes without downloading
msucat get KB4078130 --dry-run

# Output structured JSON for automation pipelines (Ansible, CI/CD, Packer)
msucat get KB4078130 --dry-run --json
```

### 2. Search for Updates
Search by KB number, title, or keywords:
```bash
# Search by KB number (returns first page of results)
msucat search KB5034441

# Fetch multiple pages (25 items per page)
msucat search "Windows 11" --pages 2

# Search with client-side post-filters
msucat search "Windows 11" --arch x64 --classification "Security Updates"

# Retrieve machine-readable JSON
msucat search "Windows Server 2022" --pages 2 --json
```

> **Note on Filtering**: `--arch`, `--product`, and `--classification` are client-side filters applied to the fetched results. To filter server-side on Microsoft's index, include keywords directly in your search query (e.g. `msucat search "KB5044284 x64"`).

### 3. View Update Metadata & Supersedence
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

### 4. Extract Direct Download Links & Checksums
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

### 5. Download Updates by GUID with Automatic Integrity Verification
Download `.msu` / `.cab` / `.exe` files with a progress bar and automatic SHA-256 verification:
```bash
# Download to default directory (./downloads) with SHA-256 verification
msucat download 20ff3247-bd92-4683-9094-c298e9c6125f

# Custom destination and filename filter
msucat download 20ff3247-bd92-4683-9094-c298e9c6125f --output /tmp/updates --filter "x64"

# Skip hash verification
msucat download 20ff3247-bd92-4683-9094-c298e9c6125f --no-verify
```

Downloads stream into a temporary `.part` file (`<filename>.part`). Upon completion, the file's SHA-256 is validated against Microsoft's official manifest. If valid, the file is atomically renamed to its destination. If corrupted or interrupted, the partial file is safely cleaned up. Existing files with matching hashes are skipped automatically.

---

## Library Usage (Rust)

Add `msucat` to your `Cargo.toml`:
```toml
[dependencies]
msucat = "0.1.2"
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

## Disclaimer

`msucat` is an independent, open-source tool and scraper for Microsoft's legacy Update Catalog WebForms portal. It is not affiliated with, sponsored by, or endorsed by Microsoft Corporation. Because the Catalog web interface is subject to change at any time without notice, this software is provided "as is", without warranty or SLA of any kind.

---

## License

Licensed under the [MIT License](LICENSE).
