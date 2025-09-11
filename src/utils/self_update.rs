use crate::utils::dialogs::{popup_error, popup_info};
use self_update::backends::github::Update;
use std::error::Error;

pub fn self_update() {
    match try_update() {
        Ok((is_new_version, version)) => if is_new_version {
            popup_info(format!("New version has been installed: {}", version))
        },
        Err(e) => popup_error(format!("Update failed: {}", e)),
    };
}

fn try_update() -> Result<(bool, String), Box<dyn Error>> {
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
    Ok((new_version > version, new_version.to_string()))
}
