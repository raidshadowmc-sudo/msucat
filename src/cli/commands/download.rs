use std::path::PathBuf;

use clap::Args;
use colored::Colorize;
use msucat::{MsuClient, Result};

use crate::cli::output::create_download_progress_bar;

#[derive(Args, Debug, Clone)]
pub struct DownloadArgs {
    /// One or more Update GUIDs
    #[arg(required = true)]
    pub update_ids: Vec<String>,

    /// Target output directory for downloaded files
    #[arg(short, long, default_value = "./downloads")]
    pub output: PathBuf,

    /// Filter download filenames by keyword/substring
    #[arg(short, long)]
    pub filter: Option<String>,

    /// Disable on-the-fly SHA-256 checksum verification
    #[arg(long = "no-verify", action = clap::ArgAction::SetFalse, default_value_t = true)]
    pub verify: bool,
}

pub async fn execute(client: &MsuClient, args: DownloadArgs) -> Result<()> {
    let id_refs: Vec<&str> = args.update_ids.iter().map(|s| s.as_str()).collect();
    let mut files = client.get_download_files_batch(&id_refs).await?;

    if let Some(ref f) = args.filter {
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

    tokio::fs::create_dir_all(&args.output).await?;
    println!(
        "{} Downloading {} file(s) to {}\n",
        "msucat:".cyan().bold(),
        files.len().to_string().green().bold(),
        args.output.display().to_string().bold()
    );

    for file in &files {
        let dest = args.output.join(&file.file_name);
        println!("{} {}", "Downloading:".cyan().bold(), file.file_name);

        let pb = create_download_progress_bar();
        let pb_clone = pb.clone();
        let expected_hash = if args.verify {
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

    println!(
        "{} All downloads completed successfully!",
        "msucat:".green().bold()
    );

    Ok(())
}
