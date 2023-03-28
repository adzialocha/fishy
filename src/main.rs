mod commands;
mod context;
mod files;
mod schema;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use commands::{init, publish, update};
use p2panda_rs::test_utils::memory_store::MemoryStore;

use context::Context;

/// A fictional versioning CLI.
#[derive(Debug, Parser)]
#[command(name = "fishy")]
#[command(about = "Create, update or install p2panda schemas")]
struct Cli {
    #[arg(short, long = "schema", default_value = "schema.toml")]
    schema_path: PathBuf,

    #[arg(short, long = "lock", default_value = "schema.lock")]
    lock_path: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialise a schema folder.
    #[command()]
    Init {
        #[arg(default_value = "my_schema")]
        name: String,
    },

    /// Create or update schema.
    #[command()]
    Update {
        #[arg(short = 'k', long = "key", default_value = "secret.txt")]
        private_key_path: PathBuf,
    },

    /// Deploy schema on a node.
    #[command()]
    Publish {
        #[arg(short, long, default_value = "http://localhost:2020/graphql")]
        endpoint: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let store = MemoryStore::default();
    let context = Context::new(store, &args.schema_path, &args.lock_path);

    match args.command {
        Commands::Init { name } => {
            init(context, &name)?;
        }
        Commands::Update { private_key_path } => {
            update(context, &private_key_path).await?;
        }
        Commands::Publish { endpoint } => publish(context, &endpoint).await?,
    };

    Ok(())
}
