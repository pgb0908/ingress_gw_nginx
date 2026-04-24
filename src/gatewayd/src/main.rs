mod models;
mod nginx;
mod paths;
mod providers;
mod revision;
mod runtime;
mod server;
mod state;

use anyhow::Result;
use clap::{Parser, Subcommand};
use runtime::GatewayRuntime;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gatewayd")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    ValidateRevision { #[arg(long)] revision_path: PathBuf },
    ActivateRevision { #[arg(long)] revision_path: PathBuf },
    Rollback,
    Status,
    ServeAdmin {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 19080)]
        port: u16,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let runtime = GatewayRuntime::new();

    match cli.command {
        Commands::ValidateRevision { revision_path } => {
            let result = runtime.validate_revision(&revision_path)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            std::process::exit(if result.valid { 0 } else { 1 });
        }
        Commands::ActivateRevision { revision_path } => {
            let result = runtime.activate_revision(&revision_path)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            std::process::exit(if result.status == "activated" { 0 } else { 1 });
        }
        Commands::Rollback => {
            let result = runtime.rollback()?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            std::process::exit(if result.status == "activated" || result.status == "rolled_back" {
                0
            } else {
                1
            });
        }
        Commands::Status => {
            let state = state::load_state()?;
            println!("{}", serde_json::to_string_pretty(&state)?);
        }
        Commands::ServeAdmin { host, port } => {
            server::serve_admin(&host, port)?;
        }
    }
    Ok(())
}

