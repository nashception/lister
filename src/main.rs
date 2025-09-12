#![windows_subsystem = "windows"]

use lister::infrastructure::updater::self_update::self_update;
use lister::ui::app::ListerApp;
use lister::ui::app_factory::ListerAppService;

fn main() -> iced::Result {
    self_update();

    let service = ListerAppService::create();
    iced::application(ListerApp::title, ListerApp::update, ListerApp::view)
        .subscription(ListerApp::subscription)
        .window(ListerApp::window())
        .run_with(|| ListerApp::new(service))
}
