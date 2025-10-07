use crate::domain::entities::directory::DirectoryData;
use crate::utils::dialogs::popup_error_and_exit;
use std::path::Path;
use sysinfo::{DiskRefreshKind, Disks};

#[must_use]
pub fn directory_data(directory: &Path) -> DirectoryData {
    let disks = Disks::new_with_refreshed_list_specifics(DiskRefreshKind::with_storage(
        DiskRefreshKind::default(),
    ));

    let disk = disks
        .iter()
        .find(|disk| directory.starts_with(disk.mount_point()))
        .unwrap_or_else(|| {
            popup_error_and_exit(format!(
                "Cannot find the disk for directory {}", directory.display()))
        });

    DirectoryData {
        drive_name: disk.name().to_string_lossy().to_string(),
        drive_available_space: disk.available_space(),
        directory: directory.to_path_buf(),
    }
}
