use anyhow::Result;
use clap::{Parser, Subcommand};
use gatewayd::{server, state};

#[derive(Parser)]
#[command(name = "gatewayd")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

    match cli.command {
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
