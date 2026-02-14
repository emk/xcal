//! Xcaliber Mark II — Rust edition.

mod app;
mod conferences;
mod help;
mod lang;
mod matching;
mod messages;
mod users;

use std::net::SocketAddr;

use app::XcaliberApp;
use clap::{Parser, Subcommand};
use miette::Report;
use tokio::net::TcpListener;
use tracing::info;

#[derive(Parser)]
#[command(name = "xcalr")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the conference server.
    Serve {
        /// TCP address to listen on.
        #[arg(long = "tcp")]
        tcp: SocketAddr,
    },
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    tracing_subscriber::EnvFilter::new(
                        "dcts=info,xcalr=info,warn",
                    )
                }),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { tcp } => {
            let listener = TcpListener::bind(tcp)
                .await
                .map_err(|e| miette::miette!("failed to bind {tcp}: {e}"))?;
            info!("listening on {tcp}");

            let mut app = XcaliberApp::new();
            let sink = app.port_message_sink();

            tokio::spawn(dcts::ports::tcp::listen(listener, sink));

            app.run().await?;
        }
    }

    Ok(())
}
