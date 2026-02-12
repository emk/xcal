mod connection;
mod diff;
mod normalize;
mod runner;
mod telnet;

use std::{
    collections::HashMap,
    fs::{self, create_dir_all, read_to_string},
    path::{Path, PathBuf},
    process,
};

use clap::{Parser, Subcommand};
use miette::{miette, IntoDiagnostic, Result};
use owo_colors::OwoColorize;
use tracing::debug;
use tracing_subscriber::EnvFilter;
use xcal_test_tools::parse_script;

use crate::{diff::diff_transcript, normalize::Normalizer, runner::Runner};

#[derive(Parser)]
#[command(
    name = "xcal-test",
    about = "Test runner for Xcaliber Mark II and compatible servers."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Server connection arguments shared across subcommands.
#[derive(clap::Args)]
struct ConnectArgs {
    /// Server host.
    #[arg(long, default_value = "localhost")]
    host: String,

    /// Server port.
    #[arg(short, long, default_value_t = 2456)]
    port: u16,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a JSONL test script against a server.
    Run {
        /// Path to the JSONL script file.
        script: PathBuf,

        #[command(flatten)]
        connect: ConnectArgs,

        /// Directory to save per-connection transcripts (default: script's
        /// parent directory).
        #[arg(long)]
        transcript_dir: Option<PathBuf>,

        /// Print transcripts to stdout instead of saving to files.
        #[arg(long, conflicts_with = "transcript_dir")]
        stdout: bool,
    },

    /// Re-run a script and diff output against saved transcripts.
    Check {
        /// Path to the JSONL script file.
        script: PathBuf,

        #[command(flatten)]
        connect: ConnectArgs,

        /// Directory containing expected transcripts (default: script's
        /// parent directory).
        #[arg(long)]
        transcript_dir: Option<PathBuf>,

        /// Path to normalize.toml rules file (default: normalize.toml in
        /// transcript dir).
        #[arg(long)]
        normalize_rules: Option<PathBuf>,
    },

    /// Check all fixtures in a directory against saved transcripts.
    CheckAll {
        /// Directory containing fixture subdirectories.
        fixtures_dir: PathBuf,

        #[command(flatten)]
        connect: ConnectArgs,
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
            connect,
            transcript_dir,
            stdout,
        } => {
            let save_dir = if stdout {
                None
            } else {
                Some(resolve_transcript_dir(transcript_dir.as_deref(), &script))
            };
            run_script(script, connect.host, connect.port, save_dir).await?;
        }
        Commands::Check {
            script,
            connect,
            transcript_dir,
            normalize_rules,
        } => {
            let exit_code = check_script(
                script,
                connect.host,
                connect.port,
                transcript_dir,
                normalize_rules,
                true,
            )
            .await?;
            if exit_code != 0 {
                process::exit(exit_code);
            }
        }
        Commands::CheckAll {
            fixtures_dir,
            connect,
        } => {
            let exit_code =
                check_all_fixtures(fixtures_dir, connect.host, connect.port)
                    .await?;
            if exit_code != 0 {
                process::exit(exit_code);
            }
        }
    }

    Ok(())
}

/// Resolve transcript directory from explicit flag or script's parent.
fn resolve_transcript_dir(explicit: Option<&Path>, script: &Path) -> PathBuf {
    explicit.map(Path::to_path_buf).unwrap_or_else(|| {
        script
            .parent()
            .expect("script path has no parent directory")
            .to_path_buf()
    })
}

/// Read all `.txt` files in a directory, keyed by file stem.
fn find_transcript_files(dir: &Path) -> Result<HashMap<String, String>> {
    let mut transcripts = HashMap::new();
    let entries = fs::read_dir(dir).into_diagnostic()?;
    for entry in entries {
        let entry = entry.into_diagnostic()?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("txt") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let content = fs::read_to_string(&path).into_diagnostic()?;
                transcripts.insert(stem.to_string(), content);
            }
        }
    }
    Ok(transcripts)
}

/// Find fixture subdirectories containing `in.jsonl`, sorted by name.
fn discover_fixtures(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut fixtures = Vec::new();
    let entries = fs::read_dir(dir).into_diagnostic()?;
    for entry in entries {
        let entry = entry.into_diagnostic()?;
        let path = entry.path();
        if path.is_dir() && path.join("in.jsonl").exists() {
            fixtures.push(path);
        }
    }
    fixtures.sort();
    Ok(fixtures)
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

async fn check_script(
    script_path: PathBuf,
    host: String,
    port: u16,
    transcript_dir: Option<PathBuf>,
    normalize_rules: Option<PathBuf>,
    verbose: bool,
) -> Result<i32> {
    let tx_dir =
        resolve_transcript_dir(transcript_dir.as_deref(), &script_path);

    // Load normalizer.
    let norm_path =
        normalize_rules.unwrap_or_else(|| tx_dir.join("normalize.toml"));
    let normalizer = Normalizer::from_path(&norm_path)?;

    // Read expected transcripts.
    let expected = find_transcript_files(&tx_dir)?;
    if expected.is_empty() {
        return Err(miette!(
            "no .txt transcript files found in {}",
            tx_dir.display()
        ));
    }

    // Run the script.
    debug!("loading script from {}", script_path.display());
    let content = read_to_string(&script_path).into_diagnostic()?;
    let lines = parse_script(&content).into_diagnostic()?;
    debug!("parsed {} script lines", lines.len());

    let mut runner = Runner::new(host, port);
    let actual: HashMap<String, String> =
        runner.run(&lines).await?.into_iter().collect();

    // Check for connection set mismatches.
    let expected_keys: std::collections::BTreeSet<_> =
        expected.keys().collect();
    let actual_keys: std::collections::BTreeSet<_> = actual.keys().collect();

    if expected_keys != actual_keys {
        let missing: Vec<_> = expected_keys.difference(&actual_keys).collect();
        let extra: Vec<_> = actual_keys.difference(&expected_keys).collect();
        let mut msg = String::from("connection set mismatch:");
        if !missing.is_empty() {
            msg.push_str(&format!(" missing={missing:?}"));
        }
        if !extra.is_empty() {
            msg.push_str(&format!(" extra={extra:?}"));
        }
        return Err(miette!("{msg}"));
    }

    // Diff each connection.
    let mut any_failed = false;
    let mut conn_names: Vec<_> = expected.keys().collect();
    conn_names.sort();

    for conn in conn_names {
        let exp_text = normalizer.apply(&expected[conn]);
        let act_text = normalizer.apply(&actual[conn]);
        let result = diff_transcript(conn, &exp_text, &act_text);
        if result.matches {
            if verbose {
                eprintln!("{} {conn}", "  ok".green());
            }
        } else {
            eprintln!("{} {conn}", "FAIL".red());
            eprint!("{}", result.diff_output);
            any_failed = true;
        }
    }

    Ok(if any_failed { 1 } else { 0 })
}

async fn check_all_fixtures(
    fixtures_dir: PathBuf,
    host: String,
    port: u16,
) -> Result<i32> {
    let fixtures = discover_fixtures(&fixtures_dir)?;
    if fixtures.is_empty() {
        return Err(miette!(
            "no fixture directories (with in.jsonl) found in {}",
            fixtures_dir.display()
        ));
    }

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    for fixture_dir in &fixtures {
        let name = fixture_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");

        if fixture_dir.join("SKIP.md").exists() {
            eprintln!("{} {name}", "skip".yellow());
            skipped += 1;
            continue;
        }

        let script_path = fixture_dir.join("in.jsonl");
        let result =
            check_script(script_path, host.clone(), port, None, None, false)
                .await;

        match result {
            Ok(0) => {
                eprintln!("{} {name}", "  ok".green());
                passed += 1;
            }
            Ok(_) => {
                eprintln!("{} {name}", "FAIL".red());
                failed += 1;
            }
            Err(e) => {
                eprintln!("{} {name}: {e}", "ERR ".red());
                failed += 1;
            }
        }
    }

    let total = passed + failed + skipped;
    eprintln!();
    eprintln!(
        "{total} fixtures: {passed} passed, {failed} failed, {skipped} skipped"
    );

    Ok(if failed > 0 { 1 } else { 0 })
}
