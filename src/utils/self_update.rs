use crate::utils::dialogs::{popup_error, popup_info};
use self_update::backends::github::Update;
use std::error::Error;

pub fn self_update() {
    match try_update() {
        Ok(new_version) => popup_info(format!("New version has been installed: {}", new_version)),
        Err(e) => popup_error(format!("Update failed: {}", e)),
    };
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

    Ok(status.version().to_string())
}
