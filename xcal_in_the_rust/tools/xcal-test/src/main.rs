mod connection;
mod runner;
mod telnet;

use std::{
    fs::{self, create_dir_all, read_to_string},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use miette::{IntoDiagnostic, Result};
use tracing::debug;
use tracing_subscriber::EnvFilter;
use xcal_test_tools::parse_script;

use crate::runner::Runner;

#[derive(Parser)]
#[command(
    name = "xcal-test",
    about = "Test runner for Xcaliber Mark II and compatible servers."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a JSONL test script against a server.
    Run {
        /// Path to the JSONL script file.
        #[arg(long)]
        script: PathBuf,

        /// Server host.
        #[arg(long, default_value = "localhost")]
        host: String,

        /// Server port.
        #[arg(short, long, default_value_t = 2456)]
        port: u16,

        /// Directory to save per-connection transcripts.
        #[arg(long)]
        save_transcripts: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let log_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("xcal_test=info,xcal_test_tools=info,warn")
    });

    tracing_subscriber::fmt().with_env_filter(log_filter).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            script,
            host,
            port,
            save_transcripts,
        } => run_script(script, host, port, save_transcripts).await?,
    }

    Ok(())
}

async fn run_script(
    script_path: PathBuf,
    host: String,
    port: u16,
    save_dir: Option<PathBuf>,
) -> Result<()> {
    debug!("loading script from {}", script_path.display());
    let content = read_to_string(&script_path).into_diagnostic()?;
    let lines = parse_script(&content).into_diagnostic()?;
    debug!("parsed {} script lines", lines.len());

    let mut runner = Runner::new(host, port);
    let transcripts = runner.run(&lines).await?;

    if let Some(dir) = save_dir {
        create_dir_all(&dir).into_diagnostic()?;
        for (name, transcript) in &transcripts {
            let path = dir.join(format!("{name}.txt"));
            fs::write(&path, transcript).into_diagnostic()?;
            debug!("saved transcript: {}", path.display());
        }
    } else {
        for (name, transcript) in &transcripts {
            println!("=== {name} ===");
            println!("{transcript}");
        }
    }

    Ok(())
}
