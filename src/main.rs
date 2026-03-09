#![windows_subsystem = "windows"]

use lister::infrastructure::updater::app_updater::self_update;
use lister::ui::app::ListerApp;
use lister::ui::app_factory::ListerAppService;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("This application only supports Linux, macOS and Windows");

fn main() -> iced::Result {
    self_update();

    iced::application(|| ListerApp::new(ListerAppService::create()), ListerApp::update, ListerApp::view)
        .subscription(|state| state.subscription())
        .window(ListerApp::window())
        .run()
}
