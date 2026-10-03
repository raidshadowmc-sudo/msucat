use clap::Args;
use colored::Colorize;
use msucat::{MsuClient, Result};

#[derive(Args, Debug, Clone)]
pub struct LinksArgs {
    /// One or more Update GUIDs
    #[arg(required = true)]
    pub update_ids: Vec<String>,

    /// Output links as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn execute(client: &MsuClient, args: LinksArgs) -> Result<()> {
    let id_refs: Vec<&str> = args.update_ids.iter().map(|s| s.as_str()).collect();
    let files = client.get_download_files_batch(&id_refs).await?;

    if args.json {
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

    Ok(())
}
