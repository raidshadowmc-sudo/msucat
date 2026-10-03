mod cli;

use clap::Parser;
use colored::Colorize;
use msucat::MsuClient;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();
    let client = MsuClient::new();

    if let Err(err) = cli::run(cli, &client).await {
        eprintln!("{} {}", "error:".red().bold(), err);
        std::process::exit(1);
    }
}
