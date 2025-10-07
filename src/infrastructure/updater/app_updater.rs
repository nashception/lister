use crate::utils::dialogs::{popup_error, popup_info};
use self_update::backends::github::Update;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::process::{exit, Command};

pub fn self_update() {
    let exe_path = match env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            popup_error(format!("Failed to get current exe path: {e}"));
            return;
        }
    };
    let just_updated = env::args().any(|arg| arg == "--updated");

    if !just_updated {
        match try_update() {
            Ok(new_version) => {
                if !new_version.is_empty() {
                    popup_info(format!("New version has been installed: {new_version}"));
                    if let Err(e) = restart(exe_path) {
                        popup_error(format!("Failed to restart: {e}"));
                    }
                }
            }
            Err(e) => popup_error(format!("Update failed: {e}")),
        }
    }
}

fn try_update() -> Result<String, Box<dyn Error>> {
    let status = Update::configure()
        .repo_owner("nashception")
        .repo_name("lister")
        .bin_name("lister")
        .current_version(env!("CARGO_PKG_VERSION"))
        .no_confirm(true)
        .build()?
        .update()?;

    let new_version = if status.updated() {
        String::from(status.version())
    } else {
        String::new()
    };

    Ok(new_version)
}

fn restart(exe_path: PathBuf) -> Result<(), Box<dyn Error>> {
    Command::new(exe_path).arg("--updated").spawn()?;

    exit(0);
}
