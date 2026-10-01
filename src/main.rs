use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use msucat::{
    DownloadFile, MsuCatError, MsuClient, Result, UpdateDetails, UpdateSummary, parse_catalog_date,
};
use std::path::PathBuf;

fn is_guid(s: &str) -> bool {
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

#[derive(Parser)]
#[command(
    name = "msucat",
    about = "Cross-platform CLI & scraper for Microsoft Update Catalog",
    version = env!("CARGO_PKG_VERSION"),
    author = "raidshadowmc-sudo"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// One-shot search, filter, link resolution, and download
    Get {
        /// Search query (KB number, product name, or GUID)
        query: String,

        /// Filter by architecture (e.g. x64, arm64, x86)
        #[arg(short, long)]
        arch: Option<String>,

        /// Filter by product name (e.g. "Windows 11", "Server 2022")
        #[arg(short, long)]
        product: Option<String>,

        /// Filter by classification (e.g. "Security Updates", "Critical Updates")
        #[arg(short = 'c', long = "class", alias = "classification")]
        class: Option<String>,

        /// Automatically select the newest update if multiple match
        #[arg(short, long)]
        latest: bool,

        /// Number of catalog pages to scan (25 items per page)
        #[arg(short = 'n', long, default_value = "3")]
        pages: usize,

        /// Target output directory for downloaded files
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// Disable on-the-fly SHA-256 checksum verification
        #[arg(long = "no-verify", action = clap::ArgAction::SetFalse, default_value_t = true)]
        verify: bool,

        /// Inspect and display resolved download URLs and hashes without downloading
        #[arg(long)]
        dry_run: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// Search the catalog for updates
    Search {
        /// Search query (KB number, product name, or keyword)
        query: String,

        /// Client-side post-filter by architecture (e.g. x64, arm64, x86)
        #[arg(short, long)]
        arch: Option<String>,

        /// Client-side post-filter by product name (e.g. "Windows 11", "Server 2022")
        #[arg(short, long)]
        product: Option<String>,

        /// Client-side post-filter by classification (e.g. "Security Updates", "Critical Updates")
        #[arg(short, long)]
        classification: Option<String>,

        /// Number of catalog pages to fetch (25 items per page)
        #[arg(short = 'n', long, default_value = "1")]
        pages: usize,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// Inspect detailed metadata for a specific update
    Info {
        /// Update GUID (e.g. 20ff3247-bd92-4683-9094-c298e9c6125f)
        update_id: String,

        /// Output details as JSON
        #[arg(long)]
        json: bool,
    },

    /// Retrieve direct CDN download URLs and verified hashes
    Links {
        /// One or more Update GUIDs
        #[arg(required = true)]
        update_ids: Vec<String>,

        /// Output links as JSON
        #[arg(long)]
        json: bool,
    },

    /// Download update files (.msu, .cab, .exe) with verified checksums
    Download {
        /// One or more Update GUIDs
        #[arg(required = true)]
        update_ids: Vec<String>,

        /// Target output directory for downloaded files
        #[arg(short, long, default_value = "./downloads")]
        output: PathBuf,

        /// Filter download filenames by keyword/substring
        #[arg(short, long)]
        filter: Option<String>,

        /// Disable on-the-fly SHA-256 checksum verification
        #[arg(long = "no-verify", action = clap::ArgAction::SetFalse, default_value_t = true)]
        verify: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let client = MsuClient::new();

    if let Err(err) = run(cli, &client).await {
        eprintln!("{} {}", "error:".red().bold(), err);
        std::process::exit(1);
    }
}

async fn run(cli: Cli, client: &MsuClient) -> Result<()> {
    match cli.command {
        Commands::Get {
            query,
            arch,
            product,
            class,
            latest,
            pages,
            output,
            verify,
            dry_run,
            json,
        } => {
            let selected: UpdateSummary;

            if is_guid(&query) {
                let details = client.get_details(&query).await.ok();
                selected = UpdateSummary {
                    id: query.clone(),
                    title: details
                        .as_ref()
                        .map(|d| d.title.clone())
                        .unwrap_or_else(|| query.clone()),
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
                let mut results = client.search_with_limit(&query, pages).await?;

                if let Some(ref a) = arch {
                    let a_lower = a.to_lowercase();
                    results.retain(|item| {
                        item.title.to_lowercase().contains(&a_lower)
                            || item.products.to_lowercase().contains(&a_lower)
                    });
                }

                if let Some(ref p) = product {
                    let p_lower = p.to_lowercase();
                    results.retain(|item| {
                        item.products.to_lowercase().contains(&p_lower)
                            || item.title.to_lowercase().contains(&p_lower)
                    });
                }

                if let Some(ref c) = class {
                    let c_lower = c.to_lowercase();
                    results.retain(|item| item.classification.to_lowercase().contains(&c_lower));
                }

                if results.is_empty() {
                    return Err(MsuCatError::NotFound(format!(
                        "No updates matching '{}' with specified filters",
                        query
                    )));
                }

                results.sort_by(|a, b| {
                    parse_catalog_date(&b.last_updated).cmp(&parse_catalog_date(&a.last_updated))
                });

                if latest || results.len() == 1 {
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

            if json {
                #[derive(serde::Serialize)]
                struct GetJsonResponse<'a> {
                    update: &'a UpdateSummary,
                    files: &'a [DownloadFile],
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&GetJsonResponse {
                        update: &selected,
                        files: &files,
                    })?
                );
                if dry_run {
                    return Ok(());
                }
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

                if dry_run {
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
                    return Ok(());
                }
            }

            // Perform downloads
            tokio::fs::create_dir_all(&output).await?;
            if !json {
                println!(
                    "{} Downloading {} file(s) to {}\n",
                    "msucat:".cyan().bold(),
                    files.len().to_string().green().bold(),
                    output.display().to_string().bold()
                );
            }

            for file in &files {
                let dest = output.join(&file.file_name);
                if !json {
                    println!("{} {}", "Downloading:".cyan().bold(), file.file_name);
                }

                let pb = if !json {
                    let pb = ProgressBar::new(0);
                    pb.set_style(
                        ProgressStyle::default_bar()
                            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                            .expect("valid template")
                            .progress_chars("#>-"),
                    );
                    Some(pb)
                } else {
                    None
                };

                let pb_clone = pb.clone();
                let expected_hash = if verify {
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

                if let Some(p) = pb {
                    p.finish_with_message("Done");
                    println!(
                        "   {} Saved to {}",
                        "✓".green().bold(),
                        downloaded_path.display()
                    );
                    if verify {
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

            if !json {
                println!(
                    "{} All downloads completed successfully!",
                    "msucat:".green().bold()
                );
            }
        }

        Commands::Search {
            query,
            arch,
            product,
            classification,
            pages,
            json,
        } => {
            let mut results = client.search_with_limit(&query, pages).await?;

            // Apply optional client-side filters
            if let Some(ref a) = arch {
                let a_lower = a.to_lowercase();
                results.retain(|item| {
                    item.title.to_lowercase().contains(&a_lower)
                        || item.products.to_lowercase().contains(&a_lower)
                });
            }

            if let Some(ref p) = product {
                let p_lower = p.to_lowercase();
                results.retain(|item| {
                    item.products.to_lowercase().contains(&p_lower)
                        || item.title.to_lowercase().contains(&p_lower)
                });
            }

            if let Some(ref c) = classification {
                let c_lower = c.to_lowercase();
                results.retain(|item| item.classification.to_lowercase().contains(&c_lower));
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&results)?);
                return Ok(());
            }

            if results.is_empty() {
                println!(
                    "{} No updates found matching '{}'",
                    "info:".yellow().bold(),
                    query
                );
                return Ok(());
            }

            println!(
                "{} Found {} updates for '{}':\n",
                "msucat:".cyan().bold(),
                results.len().to_string().green().bold(),
                query
            );

            for (i, item) in results.iter().enumerate() {
                println!(
                    "{}. {} {}",
                    (i + 1).to_string().dimmed(),
                    item.title.bold(),
                    format!("({})", item.size).yellow()
                );
                println!("   {} {}", "ID:".dimmed(), item.id.cyan());
                println!("   {} {}", "Products:".dimmed(), item.products);
                println!(
                    "   {} {} | {} {}",
                    "Classification:".dimmed(),
                    item.classification,
                    "Updated:".dimmed(),
                    item.last_updated
                );
                println!();
            }
        }

        Commands::Info { update_id, json } => {
            let details = client.get_details(&update_id).await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&details)?);
                return Ok(());
            }

            print_update_details(&details);
        }

        Commands::Links { update_ids, json } => {
            let id_refs: Vec<&str> = update_ids.iter().map(|s| s.as_str()).collect();
            let files = client.get_download_files_batch(&id_refs).await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&files)?);
                return Ok(());
            }

            println!(
                "{} Retrieved {} download file(s):\n",
                "msucat:".cyan().bold(),
                files.len().to_string().green().bold()
            );

            for (i, file) in files.iter().enumerate() {
                println!(
                    "{}. {}",
                    (i + 1).to_string().dimmed(),
                    file.file_name.bold()
                );
                if !file.title.is_empty() {
                    println!("   {} {}", "Title:".dimmed(), file.title);
                }
                println!("   {} {}", "URL:".dimmed(), file.url.cyan());
                if let Some(ref sha256) = file.sha256_hex {
                    println!("   {} {}", "SHA-256:".dimmed(), sha256.green());
                }
                if let Some(ref sha1) = file.sha1_hex {
                    println!("   {} {}", "SHA-1:".dimmed(), sha1.yellow());
                }
                println!();
            }
        }

        Commands::Download {
            update_ids,
            output,
            filter,
            verify,
        } => {
            let id_refs: Vec<&str> = update_ids.iter().map(|s| s.as_str()).collect();
            let mut files = client.get_download_files_batch(&id_refs).await?;

            if let Some(ref f) = filter {
                let f_lower = f.to_lowercase();
                files.retain(|file| {
                    file.file_name.to_lowercase().contains(&f_lower)
                        || file.url.to_lowercase().contains(&f_lower)
                });
            }

            if files.is_empty() {
                println!(
                    "{} No files matched download criteria.",
                    "warning:".yellow().bold()
                );
                return Ok(());
            }

            tokio::fs::create_dir_all(&output).await?;
            println!(
                "{} Downloading {} file(s) to {}\n",
                "msucat:".cyan().bold(),
                files.len().to_string().green().bold(),
                output.display().to_string().bold()
            );

            for file in &files {
                let dest = output.join(&file.file_name);
                println!("{} {}", "Downloading:".cyan().bold(), file.file_name);

                let pb = ProgressBar::new(0);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                        .expect("valid template")
                        .progress_chars("#>-"),
                );

                let pb_clone = pb.clone();
                let expected_hash = if verify {
                    file.sha256_hex.as_deref()
                } else {
                    None
                };

                let downloaded_path = client
                    .download_file(&file.url, &dest, expected_hash, move |bytes, total| {
                        if let Some(t) = total {
                            pb_clone.set_length(t);
                        }
                        pb_clone.set_position(bytes);
                    })
                    .await?;

                pb.finish_with_message("Done");
                println!(
                    "   {} Saved to {}",
                    "✓".green().bold(),
                    downloaded_path.display()
                );
                if verify {
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

            println!(
                "{} All downloads completed successfully!",
                "msucat:".green().bold()
            );
        }
    }

    Ok(())
}

fn print_update_details(details: &UpdateDetails) {
    println!("══════════════════════════════════════════════════════════════════════");
    println!("  {}", details.title.bold());
    println!("══════════════════════════════════════════════════════════════════════");
    println!("  {} {}", "Update ID     :".dimmed(), details.id.cyan());
    println!(
        "  {} {}",
        "Classification:".dimmed(),
        details.classification.yellow()
    );

    if !details.architectures.is_empty() {
        println!(
            "  {} {}",
            "Architectures :".dimmed(),
            details.architectures.join(", ")
        );
    }

    if !details.kb_numbers.is_empty() {
        println!(
            "  {} {}",
            "KB Numbers    :".dimmed(),
            details.kb_numbers.join(", ").green().bold()
        );
    }

    if !details.supported_products.is_empty() {
        println!(
            "  {} {}",
            "Products      :".dimmed(),
            details.supported_products.join(", ")
        );
    }

    if !details.supported_languages.is_empty() {
        println!(
            "  {} {}",
            "Languages     :".dimmed(),
            details.supported_languages.join(", ")
        );
    }

    if !details.msrc_number.is_empty() && details.msrc_number != "n/a" {
        println!("  {} {}", "MSRC Number   :".dimmed(), details.msrc_number);
    }

    if !details.msrc_severity.is_empty() && details.msrc_severity != "n/a" {
        println!("  {} {}", "MSRC Severity :".dimmed(), details.msrc_severity);
    }

    println!(
        "  {} {}",
        "Reboot Behavior:".dimmed(),
        details.restart_behavior
    );

    if !details.description.is_empty() {
        println!("──────────────────────────────────────────────────────────────────────");
        println!("  {}", "Description:".bold());
        println!("  {}", details.description);
    }

    if !details.superseded_by.is_empty() {
        println!("──────────────────────────────────────────────────────────────────────");
        println!(
            "  {} (replaces this update):",
            "Superseded By".yellow().bold()
        );
        for s in &details.superseded_by {
            println!("   - {}", s);
        }
    }

    if !details.supersedes.is_empty() {
        println!("──────────────────────────────────────────────────────────────────────");
        println!(
            "  {} (updates replaced by this):",
            "Supersedes".blue().bold()
        );
        for s in &details.supersedes {
            println!("   - {}", s);
        }
    }

    if !details.more_info_urls.is_empty() {
        println!("──────────────────────────────────────────────────────────────────────");
        println!("  {}", "More Information:".bold());
        for u in &details.more_info_urls {
            println!("   - {}", u.cyan());
        }
    }
    println!("══════════════════════════════════════════════════════════════════════");
}
