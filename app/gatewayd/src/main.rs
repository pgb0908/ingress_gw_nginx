use anyhow::Result;
use clap::{Parser, Subcommand};
use gatewayd::{runtime::GatewayRuntime, server, state};
use std::path::PathBuf;

fn build_sha() -> &'static str {
    option_env!("GATEWAY_BUILD_SHA").unwrap_or("unknown")
}

fn build_time() -> &'static str {
    option_env!("GATEWAY_BUILD_TIME").unwrap_or("unknown")
}

#[derive(Parser)]
#[command(name = "gatewayd")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Status,
    Version,
    ServeAdmin {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 19080)]
        port: u16,
    },
    ActivateRevision {
        #[arg(long)]
        revision_path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => {
            let state = state::load_state()?;
            println!("{}", serde_json::to_string_pretty(&state)?);
        }
        Commands::Version => {
            println!("gatewayd {}", env!("CARGO_PKG_VERSION"));
            println!("build_sha={}", build_sha());
            println!("build_time={}", build_time());
        }
        Commands::ServeAdmin { host, port } => {
            server::serve_admin(&host, port)?;
        }
        Commands::ActivateRevision { revision_path } => {
            let runtime = GatewayRuntime::new();
            let result = runtime.load_revision(&revision_path)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            if result.status != "loaded" {
                anyhow::bail!("failed to activate revision: {}", result.message);
            }
        }
    }
    Ok(())
}
