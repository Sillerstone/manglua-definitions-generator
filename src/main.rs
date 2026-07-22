mod dump;
mod generator;

use crate::dump::{download_dump, get_actual_dump_version, get_expected_dump_version};
use crate::generator::generate_definitions;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Parser)]
#[command(version, about = "MangLua definitions generator")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Generate definitions")]
    Generate {
        dump: Option<PathBuf>,
        output: Option<PathBuf>,
    },
    #[command(about = "Install & update MangLua dump")]
    Dump {
        #[command(subcommand)]
        command: DumpCommands,
    },
}

#[derive(Subcommand)]
enum DumpCommands {
    #[command(about = "Get dump version")]
    Version { target: Option<PathBuf> },
    #[command(about = "Update dump if needed")]
    Update { target: Option<PathBuf> },
    #[command(about = "Download latest dump")]
    Install { path: Option<PathBuf> },
}

#[derive(Debug)]
struct Config {
    dump_url: String,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

fn config() -> &'static Config {
    CONFIG.get().expect("config is not initialized")
}

fn main() -> Result<()> {
    CONFIG.set(Config {
        dump_url: std::env::var("MLUA_DUMP_URL").unwrap_or("https://gist.githubusercontent.com/Votond/7a7aa53f3b4a675e3e93a9842e2079ae/raw/dump.json".to_string())
    }).expect("config has been already initialized");

    let cli = Cli::parse();

    let default_dump_path = PathBuf::from("./dump.json");
    let default_output_dir = PathBuf::from("./library");
    match cli.command.unwrap_or(Commands::Generate {
        dump: Some(default_dump_path.clone()),
        output: Some(default_output_dir.clone()),
    }) {
        Commands::Generate { dump, output } => {
            let path = dump.unwrap_or(default_dump_path);
            let output_dir = output.unwrap_or(default_output_dir);
            if path.exists() {
                if output_dir.exists() && !output_dir.is_dir() {
                    anyhow::bail!("Output directory {output_dir:?} is not a directory");
                } else {
                    create_dir_all(&output_dir)
                        .context(format!("Failed to create output directory {output_dir:?}"))?;
                    println!("Generating definition files...");
                    generate_definitions(path.as_path(), output_dir.as_path()).context(format!(
                        "Failed to generate definitions in {output_dir:?} for {path:?}"
                    ))?;
                    println!("Definition files successfully generated in {output_dir:?}");
                }
            } else {
                anyhow::bail!("Dump {path:?} does not exist");
            }
        }
        Commands::Dump { command } => match command {
            DumpCommands::Version { target } => {
                let path = target.unwrap_or(default_dump_path);
                if path.exists() {
                    println!(
                        "Version of {path:?} is {}",
                        get_actual_dump_version(path.as_path())
                            .context(format!("Failed to get dump version of {path:?}"))?
                    );
                } else {
                    anyhow::bail!("Dump {path:?} does not exist");
                }
            }
            DumpCommands::Update { target } => {
                let path = target.unwrap_or(default_dump_path);
                if path.exists() {
                    println!("Checking version...");
                    let actual = get_actual_dump_version(path.as_path())
                        .context(format!("Failed to get actual dump version of {path:?}"))?;
                    let expected = get_expected_dump_version().context(format!(
                        "Failed to get expected dump version from {}",
                        config().dump_url
                    ))?;
                    if actual < expected {
                        println!(
                            "Dump is outdated (current version: {actual}; latest version: {expected}). Downloading latest version..."
                        );
                        download_dump(path.as_path()).context(format!(
                            "Failed to download dump to {path:?} from {}",
                            config().dump_url
                        ))?;
                        println!("Dump downloaded")
                    } else {
                        println!("Dump is up to date")
                    }
                } else {
                    println!("Dump {path:?} does not exist. Downloading latest version...");
                    download_dump(path.as_path()).context(format!(
                        "Failed to download dump to {path:?} from {}",
                        config().dump_url
                    ))?;
                    println!("Dump downloaded");
                }
            }
            DumpCommands::Install { path } => {
                let path = path.unwrap_or(default_dump_path);
                println!("Downloading dump...");
                download_dump(path.as_path()).context(format!(
                    "Failed to download dump to {path:?} from {}",
                    config().dump_url
                ))?;
                println!("Dump downloaded");
            }
        },
    }

    Ok(())
}
