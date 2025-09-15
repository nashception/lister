use crate::utils::dialogs::popup_error_and_exit;
use std::path::PathBuf;
use sysinfo::{DiskRefreshKind, Disks};

pub fn available_space(directory: PathBuf) -> u64 {
    Disks::new_with_refreshed_list_specifics(DiskRefreshKind::with_storage(Default::default()))
        .iter()
        .find(|disk| directory.starts_with(disk.mount_point()))
        .map(|disk| disk.available_space())
        .unwrap_or_else(|| {
            popup_error_and_exit(format!(
                "Cannot find the disk for directory {:?}",
                directory
            ))
        })
}
