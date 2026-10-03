use clap::Args;
use msucat::{MsuClient, Result};

use crate::cli::output::print_update_details;

#[derive(Args, Debug, Clone)]
pub struct InfoArgs {
    /// Update GUID (e.g. 20ff3247-bd92-4683-9094-c298e9c6125f)
    pub update_id: String,

    /// Output details as JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn execute(client: &MsuClient, args: InfoArgs) -> Result<()> {
    let details = client.get_details(&args.update_id).await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&details)?);
        return Ok(());
    }

    print_update_details(&details);
    Ok(())
}
