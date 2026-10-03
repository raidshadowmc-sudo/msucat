use clap::Args;
use colored::Colorize;
use msucat::{MsuClient, Result};

use crate::cli::helpers::matches_arch;

#[derive(Args, Debug, Clone)]
pub struct SearchArgs {
    /// Search query (KB number, product name, or keyword)
    pub query: String,

    /// Client-side post-filter by architecture (e.g. x64, arm64, x86)
    #[arg(short, long)]
    pub arch: Option<String>,

    /// Client-side post-filter by product name (e.g. "Windows 11", "Server 2022")
    #[arg(short, long)]
    pub product: Option<String>,

    /// Client-side post-filter by classification (e.g. "Security Updates", "Critical Updates")
    #[arg(short, long)]
    pub classification: Option<String>,

    /// Number of catalog pages to fetch (25 items per page)
    #[arg(short = 'n', long, default_value = "1")]
    pub pages: usize,

    /// Output results as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn execute(client: &MsuClient, args: SearchArgs) -> Result<()> {
    let mut results = client.search_with_limit(&args.query, args.pages).await?;

    // Apply optional client-side filters
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

    if let Some(ref c) = args.classification {
        let c_lower = c.to_lowercase();
        results.retain(|item| item.classification.to_lowercase().contains(&c_lower));
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
        return Ok(());
    }

    if results.is_empty() {
        println!(
            "{} No updates found matching '{}'",
            "info:".yellow().bold(),
            args.query
        );
        return Ok(());
    }

    println!(
        "{} Found {} updates for '{}':\n",
        "msucat:".cyan().bold(),
        results.len().to_string().green().bold(),
        args.query
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

    Ok(())
}
