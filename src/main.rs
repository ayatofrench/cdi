use std::{env, fs::File, io::Read, path::PathBuf};

use pom_config as config;
use pom_server as server;
use pom_tui as tui;

fn get_config() -> anyhow::Result<String> {
    let mut cwd: PathBuf = env::current_dir()?;
    cwd.push(".pom.kdl");

    let config_file_path: PathBuf = cwd;

    let mut file: File = File::open(&config_file_path)?;

    let mut buf: String = String::new();
    file.read_to_string(&mut buf)?;

    Ok(buf)
}

fn split_command_into_parts(input: &str) -> Option<(String, Vec<String>)> {
    let trimmed_command_str = input.trim();
    if trimmed_command_str.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    let mut current_part = String::new();
    let mut in_quotes = false;

    for char_code in trimmed_command_str.chars() {
        match char_code {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' if !in_quotes => {
                if !current_part.is_empty() {
                    parts.push(current_part.clone());
                    current_part.clear();
                }
            }
            _ => {
                current_part.push(char_code);
            }
        }
    }
    if !current_part.is_empty() {
        parts.push(current_part);
    }

    match parts.first() {
        Some(command) => {
            let args = parts.iter().skip(1).cloned().collect();

            return Some((command.to_string(), args));
        }
        None => None,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut commands_to_run = Vec::new();
    let mut services = Vec::new();
    let data = get_config()?;
    let cfg = config::Config::parse(".pom.kdl", &data)?;

    for service in cfg.services.iter() {
        if let Some(command) = split_command_into_parts(service.cmd.as_str()) {
            commands_to_run.push(command);
        }
        services.push(service.name.clone());
    }

    let conn = server::start(commands_to_run).unwrap();
    let _ = tui::run(conn, services).await;

    Ok(())
}
