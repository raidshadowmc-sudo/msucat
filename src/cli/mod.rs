use clap::{Parser, Subcommand};
use msucat::{MsuClient, Result};

pub mod commands;
pub mod helpers;
pub mod output;

use commands::{
    download::{self, DownloadArgs},
    get::{self, GetArgs},
    info::{self, InfoArgs},
    links::{self, LinksArgs},
    search::{self, SearchArgs},
};

#[derive(Parser, Debug)]
#[command(
    name = "msucat",
    about = "Cross-platform CLI & scraper for Microsoft Update Catalog",
    version = env!("CARGO_PKG_VERSION"),
    author = "raidshadowmc-sudo"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// One-shot search, filter, link resolution, and download
    Get(GetArgs),

    /// Search the catalog for updates
    Search(SearchArgs),

    /// Inspect detailed metadata for a specific update
    Info(InfoArgs),

    /// Retrieve direct CDN download URLs and verified hashes
    Links(LinksArgs),

    /// Download update files (.msu, .cab, .exe) with verified checksums
    Download(DownloadArgs),
}

/// Dispatches parsed CLI subcommands to their respective handlers.
pub async fn run(cli: Cli, client: &MsuClient) -> Result<()> {
    match cli.command {
        Commands::Get(args) => get::execute(client, args).await,
        Commands::Search(args) => search::execute(client, args).await,
        Commands::Info(args) => info::execute(client, args).await,
        Commands::Links(args) => links::execute(client, args).await,
        Commands::Download(args) => download::execute(client, args).await,
    }
}
