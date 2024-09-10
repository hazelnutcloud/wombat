use std::{env, fs, path::PathBuf};

use clap::{Args, Parser};
use dialoguer::Input;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};

#[derive(Parser)]
#[command(version, about, long_about = None, bin_name="wombat")]
pub struct Cli {
    #[arg(short, long, name = "PATH", id = "config_file")]
    config_file: Option<PathBuf>,

    #[command(flatten)]
    config_values: ConfigArgs,
}

#[derive(Args)]
#[group(multiple = true, conflicts_with = "config_file")]
struct ConfigArgs {
    #[arg(short = 'u', long, name = "URL")]
    server_url: Option<String>,

    #[arg(short = 'k', long)]
    secret_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    server_url: String,
    secret_key: String,
}

pub fn write_config(config: &Config) -> Result<()> {
    let path = if let Some(path) = get_config_path() {
        path
    } else {
        PathBuf::from(
            Input::<String>::new()
                .with_prompt("Path to write config file")
                .interact_text()
                .context("Failed to read input")?,
        )
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, toml::to_string(config)?)?;

    Ok(())
}

pub fn get_config(cli: Cli) -> Result<(Config, bool)> {
    let mut write_config_file = false;
    let config = if let Some(config_path) = cli.config_file {
        let config_file = fs::read_to_string(config_path).context("Failed to read config file")?;
        toml::from_str(&config_file).context("Failed to parse config file")?
    } else {
        if cli.config_values.server_url.is_some() && cli.config_values.secret_key.is_some() {
            Config {
                secret_key: cli.config_values.secret_key.unwrap(),
                server_url: cli.config_values.server_url.unwrap(),
            }
        } else {
            let existing_config_path = if let Some(path) = get_config_path() {
                if let Ok(exists) = path.try_exists() {
                    if exists {
                        Some(path)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let existing_config: Option<Config> =
                if let Some(existing_config_path) = existing_config_path {
                    let config_string = fs::read_to_string(existing_config_path)
                        .context("Failed to read config file")?;
                    Some(toml::from_str(&config_string).context("Failed to parse config file")?)
                } else {
                    None
                };

            let server_url = if let Some(server_url) = cli.config_values.server_url {
                server_url
            } else {
                if let Some(existing_config) = existing_config.as_ref() {
                    existing_config.server_url.clone()
                } else {
                    write_config_file = true;
                    Input::<String>::new()
                        .default("http://localhost:8080".into())
                        .with_prompt("Server URL ")
                        .interact_text()?
                        .trim_end_matches('/')
                        .into()
                }
            };

            let secret_key = if let Some(secret_key) = cli.config_values.secret_key {
                secret_key
            } else {
                if let Some(existing_config) = existing_config {
                    existing_config.secret_key
                } else {
                    write_config_file = true;

                    let signup_url = format!("{server_url}/auth/signup");
                    println!("Link your discord account at {signup_url} and enter your secret key");
                    Input::<String>::new().interact_text()?
                }
            };

            Config {
                secret_key,
                server_url,
            }
        }
    };
    Ok((config, write_config_file))
}

fn get_config_path() -> Option<PathBuf> {
    if let Ok(path) = env::var("WOMBAT_CONFIG") {
        Some(PathBuf::from(path))
    } else if let Some(project_dirs) = ProjectDirs::from("com", "hazelnutcloud", "wombat") {
        Some(project_dirs.config_dir().join("config.toml"))
    } else {
        None
    }
}
