use crate::utils::dialogs::{popup_error, popup_info};
use self_update::backends::github::Update;
use std::error::Error;

pub fn self_update() {
    match try_update() {
        Ok(message) => {
            if !message.is_empty() {
                popup_info(message)
            }
        }
        Err(e) => popup_error(format!("Update failed: {}", e)),
    }
}

fn try_update() -> Result<String, Box<dyn Error>> {
    let version = env!("CARGO_PKG_VERSION");
    let status = Update::configure()
        .repo_owner("nashception")
        .repo_name("lister")
        .bin_name("lister")
        .current_version(version)
        .no_confirm(true)
        .build()?
        .update()?;

    let new_version = status.version();
    let message = if new_version > version {
        format!("New version has been installed: {}", new_version)
    } else {
        String::default()
    };
    Ok(message)
}
