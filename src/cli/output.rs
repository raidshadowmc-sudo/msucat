use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use msucat::UpdateDetails;

/// Formats and displays comprehensive metadata for an update in a clean, human-readable card format.
pub fn print_update_details(details: &UpdateDetails) {
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

/// Creates a standardized progress bar for tracking download operations.
pub fn create_download_progress_bar() -> ProgressBar {
    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .expect("valid template")
            .progress_chars("#>-"),
    );
    pb
}
