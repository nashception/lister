use rfd::{MessageButtons, MessageDialog, MessageLevel};
use std::fmt::Display;
use std::process::exit;

pub fn popup_error(error: impl Display) {
    MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title("An error happened")
        .set_description(error.to_string())
        .set_buttons(MessageButtons::Ok)
        .show();
}

pub fn popup_error_and_exit(error: impl Display) -> ! {
    popup_error(error);
    exit(1)
}
