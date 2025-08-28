#![windows_subsystem = "windows"]
extern crate libsqlite3_sys;

use lister::ui::app::ListerApp;

fn main() -> iced::Result {
    iced::application(ListerApp::title, ListerApp::update, ListerApp::view)
        .subscription(ListerApp::subscription)
        .window(ListerApp::window())
        .run_with(ListerApp::new)
}
