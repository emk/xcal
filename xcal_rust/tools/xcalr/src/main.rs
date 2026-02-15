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
        /// TCP address to listen on (default: 127.0.0.1:2456).
        #[arg(long = "tcp", default_missing_value = "127.0.0.1:2456", num_args = 0..=1, require_equals = true)]
        tcp: Option<SocketAddr>,
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
            let addr = tcp.unwrap_or_else(|| {
                #[allow(clippy::expect_used)]
                "127.0.0.1:2456".parse().expect("valid default address")
            });
            let listener = TcpListener::bind(addr)
                .await
                .map_err(|e| miette::miette!("failed to bind {addr}: {e}"))?;
            info!("listening on {addr}");

            let mut app = XcaliberApp::new();
            let sink = app.port_message_sink();

            tokio::spawn(dcts::ports::tcp::listen(listener, sink));

            app.run().await?;
        }
    }

    Ok(())
}
