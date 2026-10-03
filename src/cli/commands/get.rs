use std::path::PathBuf;

use clap::Args;
use colored::Colorize;
use msucat::{DownloadFile, MsuCatError, MsuClient, Result, UpdateSummary, parse_catalog_date};
use serde::Serialize;

use crate::cli::helpers::{is_guid, matches_arch, resolve_latest};
use crate::cli::output::create_download_progress_bar;

#[derive(Args, Debug, Clone)]
pub struct GetArgs {
    /// Search query (KB number, product name, or GUID)
    pub query: String,

    /// Filter by architecture (e.g. x64, arm64, x86)
    #[arg(short, long)]
    pub arch: Option<String>,

    /// Filter by product name (e.g. "Windows 11", "Server 2022")
    #[arg(short, long)]
    pub product: Option<String>,

    /// Filter by classification (e.g. "Security Updates", "Critical Updates")
    #[arg(short = 'c', long = "class", alias = "classification")]
    pub class: Option<String>,

    /// Automatically select the newest update if multiple match
    #[arg(short, long)]
    pub latest: bool,

    /// Number of catalog pages to scan (25 items per page)
    #[arg(short = 'n', long, default_value = "3")]
    pub pages: usize,

    /// Target output directory for downloaded files
    #[arg(short, long, default_value = ".")]
    pub output: PathBuf,

    /// Disable on-the-fly SHA-256 checksum verification
    #[arg(long = "no-verify", action = clap::ArgAction::SetFalse, default_value_t = true)]
    pub verify: bool,

    /// Inspect and display resolved download URLs and hashes without downloading
    #[arg(long)]
    pub dry_run: bool,

    /// Output results as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Serialize)]
struct GetDryRunJsonResponse<'a> {
    status: &'static str,
    update: &'a UpdateSummary,
    files: &'a [DownloadFile],
}

#[derive(Serialize)]
struct GetSuccessJsonResponse<'a> {
    status: &'static str,
    update: &'a UpdateSummary,
    files: &'a [DownloadFile],
    downloaded_files: &'a [PathBuf],
}

pub async fn execute(client: &MsuClient, args: GetArgs) -> Result<()> {
    let selected: UpdateSummary;

    if is_guid(&args.query) {
        let details = client.get_details(&args.query).await.ok();
        selected = UpdateSummary {
            id: args.query.clone(),
            title: details
                .as_ref()
                .map(|d| d.title.clone())
                .unwrap_or_else(|| args.query.clone()),
            products: details
                .as_ref()
                .map(|d| d.supported_products.join(", "))
                .unwrap_or_default(),
            classification: details
                .as_ref()
                .map(|d| d.classification.clone())
                .unwrap_or_default(),
            last_updated: String::new(),
            version: String::new(),
            size: String::new(),
            size_bytes: 0,
        };
    } else {
        let mut results = client.search_with_limit(&args.query, args.pages).await?;

        if let Some(ref a) = args.arch {
            results.retain(|item| matches_arch(&item.title, a) || matches_arch(&item.products, a));
        }

        if let Some(ref p) = args.product {
            let p_lower = p.to_lowercase();
            results.retain(|item| {
                item.products.to_lowercase().contains(&p_lower)
                    || item.title.to_lowercase().contains(&p_lower)
            });
        }

        if let Some(ref c) = args.class {
            let c_lower = c.to_lowercase();
            results.retain(|item| item.classification.to_lowercase().contains(&c_lower));
        }

        if results.is_empty() {
            return Err(MsuCatError::NotFound(format!(
                "No updates matching '{}' with specified filters",
                args.query
            )));
        }

        results.sort_by(|a, b| {
            parse_catalog_date(&b.last_updated).cmp(&parse_catalog_date(&a.last_updated))
        });

        if args.latest {
            let resolved = resolve_latest(client, &results).await?;
            selected = resolved.clone();
        } else if results.len() == 1 {
            selected = results.remove(0);
        } else {
            eprintln!(
                "{} Found {} updates matching query:\n",
                "msucat:".cyan().bold(),
                results.len().to_string().yellow().bold()
            );
            for (i, item) in results.iter().take(10).enumerate() {
                eprintln!(
                    "  {}. {} {}",
                    (i + 1).to_string().dimmed(),
                    item.title.bold(),
                    format!("({})", item.size).yellow()
                );
                eprintln!(
                    "     {} {} | {} {}",
                    "Date:".dimmed(),
                    item.last_updated,
                    "ID:".dimmed(),
                    item.id.cyan()
                );
            }
            if results.len() > 10 {
                eprintln!("  ... and {} more updates", results.len() - 10);
            }
            eprintln!();
            return Err(MsuCatError::Generic(format!(
                "Multiple updates matched (found {}). Specify --latest to automatically select the newest update, or refine query with --arch/--class/--product.",
                results.len()
            )));
        }
    }

    let files = client.get_download_files(&selected.id).await?;
    if files.is_empty() {
        return Err(MsuCatError::NotFound(format!(
            "No downloadable files found for update ID {}",
            selected.id
        )));
    }

    if args.dry_run {
        if args.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&GetDryRunJsonResponse {
                    status: "dry_run",
                    update: &selected,
                    files: &files,
                })?
            );
        } else {
            println!("{} Selected update:", "msucat:".cyan().bold());
            println!("   {} {}", "Title:".dimmed(), selected.title.bold());
            println!("   {} {}", "ID   :".dimmed(), selected.id.cyan());
            if !selected.last_updated.is_empty() || !selected.size.is_empty() {
                println!(
                    "   {} {} | {} {}",
                    "Date :".dimmed(),
                    selected.last_updated.yellow(),
                    "Size :".dimmed(),
                    selected.size.yellow()
                );
            }
            println!();

            println!(
                "{} Resolved files (dry-run, no downloads performed):",
                "msucat:".yellow().bold()
            );
            for file in &files {
                println!(" - {}", file.file_name.bold());
                println!("   {} {}", "URL   :".dimmed(), file.url.cyan());
                if let Some(ref h) = file.sha256_hex {
                    println!("   {} {}", "SHA256:".dimmed(), h);
                }
            }
        }
        return Ok(());
    }

    if !args.json {
        println!("{} Selected update:", "msucat:".cyan().bold());
        println!("   {} {}", "Title:".dimmed(), selected.title.bold());
        println!("   {} {}", "ID   :".dimmed(), selected.id.cyan());
        if !selected.last_updated.is_empty() || !selected.size.is_empty() {
            println!(
                "   {} {} | {} {}",
                "Date :".dimmed(),
                selected.last_updated.yellow(),
                "Size :".dimmed(),
                selected.size.yellow()
            );
        }
        println!();

        println!(
            "{} Downloading {} file(s) to {}\n",
            "msucat:".cyan().bold(),
            files.len().to_string().green().bold(),
            args.output.display().to_string().bold()
        );
    }

    // Perform downloads
    tokio::fs::create_dir_all(&args.output).await?;
    let mut downloaded_files = Vec::new();

    for file in &files {
        let dest = args.output.join(&file.file_name);
        if !args.json {
            println!("{} {}", "Downloading:".cyan().bold(), file.file_name);
        }

        let pb = if !args.json {
            Some(create_download_progress_bar())
        } else {
            None
        };

        let pb_clone = pb.clone();
        let expected_hash = if args.verify {
            file.sha256_hex.as_deref()
        } else {
            None
        };

        let downloaded_path = client
            .download_file(&file.url, &dest, expected_hash, move |bytes, total| {
                if let Some(ref p) = pb_clone {
                    if let Some(t) = total {
                        p.set_length(t);
                    }
                    p.set_position(bytes);
                }
            })
            .await?;

        downloaded_files.push(downloaded_path.clone());

        if let Some(p) = pb {
            p.finish_with_message("Done");
            println!(
                "   {} Saved to {}",
                "✓".green().bold(),
                downloaded_path.display()
            );
            if args.verify {
                if let Some(ref hash) = file.sha256_hex {
                    println!(
                        "   {} SHA-256 verified: {}",
                        "✓".green().bold(),
                        hash.dimmed()
                    );
                } else {
                    println!(
                        "   {} Warning: no catalog hash available, skipping verification",
                        "!".yellow().bold()
                    );
                }
            } else {
                println!(
                    "   {} Checksum verification skipped (--no-verify)",
                    "!".yellow().bold()
                );
            }
            println!();
        }
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&GetSuccessJsonResponse {
                status: "success",
                update: &selected,
                files: &files,
                downloaded_files: &downloaded_files,
            })?
        );
    } else {
        println!(
            "{} All downloads completed successfully!",
            "msucat:".green().bold()
        );
    }

    Ok(())
}
