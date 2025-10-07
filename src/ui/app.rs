use crate::domain::entities::language::Language;
use crate::tr;
use crate::ui::app_factory::ListerAppService;
use crate::ui::messages::app_message::AppMessage;
use crate::ui::pages::read_page::ReadPage;
use crate::ui::pages::write_page::WritePage;
use crate::utils::dialogs::popup_error;
use iced::keyboard::key::Named;
use iced::keyboard::Modifiers;
use iced::widget::{button, column, row, text, Space};
use iced::window::{icon, Icon, Settings};
use iced::{keyboard, widget, Alignment, Element, Length, Subscription, Task};
use std::collections::HashMap;

enum Page {
    Read(ReadPage),
    Write(WritePage),
}

pub struct ListerApp {
    service: ListerAppService,
    current_language: Language,
    translations: HashMap<String, String>,
    current_page: Page,
}

impl ListerApp {
    pub fn new(service: ListerAppService) -> (Self, Task<AppMessage>) {
        let (current_language, translations) = service.translations();

        let (read_page, task) = ReadPage::new(service.query_use_case.clone());

        (
            Self {
                service,
                current_language,
                translations,
                current_page: Page::Read(read_page),
            },
            task.map(AppMessage::Read),
        )
    }

    #[must_use]
    pub fn window() -> Settings {
        Settings {
            icon: Self::lister_icon(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn title(&self) -> String {
        format!(
            "{} (v{})",
            match &self.current_page {
                Page::Read(_) => ReadPage::title(&self.translations),
                Page::Write(_) => WritePage::title(&self.translations),
            },
            env!("CARGO_PKG_VERSION")
        )
    }

    pub fn view(&'_ self) -> Element<'_, AppMessage> {
        let language_toggle = self.language_toggle();
        let nav_bar = self.nav_bar();

        let content = match &self.current_page {
            Page::Read(page) => page
                .view(&self.translations, &self.current_language)
                .map(AppMessage::Read),
            Page::Write(page) => page.view(&self.translations).map(AppMessage::Write),
        };

        column![language_toggle, Space::with_height(10), nav_bar, content]
            .padding(20)
            .into()
    }

    pub fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::ChangeLanguage(language) => self.change_language(language),
            AppMessage::LanguageChanged(language, translations) => {
                self.current_language = language;
                self.translations = translations;
                Task::none()
            }
            AppMessage::GoToRead => {
                if matches!(self.current_page, Page::Write(_)) {
                    let (read_page, task) = ReadPage::new(self.service.query_use_case.clone());
                    self.current_page = Page::Read(read_page);
                    task.map(AppMessage::Read)
                } else {
                    Task::none()
                }
            }
            AppMessage::GoToWrite => {
                if matches!(self.current_page, Page::Read(_)) {
                    let (write_page, task) = WritePage::new(
                        self.service.indexing_use_case.clone(),
                        self.service.directory_picker.clone(),
                    );
                    self.current_page = Page::Write(write_page);
                    task.map(AppMessage::Write)
                } else {
                    Task::none()
                }
            }
            AppMessage::Read(msg) => {
                if let Page::Read(page) = &mut self.current_page {
                    page.update(msg).map(AppMessage::Read)
                } else {
                    Task::none()
                }
            }
            AppMessage::Write(msg) => {
                if let Page::Write(page) = &mut self.current_page {
                    page.update(msg).map(AppMessage::Write)
                } else {
                    Task::none()
                }
            }
            AppMessage::TabPressed { shift } => {
                if shift {
                    widget::focus_previous()
                } else {
                    widget::focus_next()
                }
            }
            AppMessage::ChangePage => match self.current_page {
                Page::Read(_) => self.update(AppMessage::GoToWrite),
                Page::Write(_) => self.update(AppMessage::GoToRead),
            },
        }
    }

    pub fn subscription(&self) -> Subscription<AppMessage> {
        let app_subscription = keyboard::on_key_press(|key, modifiers| {
            let keyboard::Key::Named(key) = key else {
                return None;
            };
            match (key, modifiers) {
                (Named::Tab, Modifiers::CTRL) => Some(AppMessage::ChangePage),
                (Named::Tab, _) => Some(AppMessage::TabPressed {
                    shift: modifiers.shift(),
                }),
                _ => None,
            }
        });
        let page_subscription = match &self.current_page {
            Page::Read(_) => ReadPage::subscription().map(AppMessage::Read),
            Page::Write(_) => Subscription::none(),
        };

        Subscription::batch(vec![app_subscription, page_subscription])
    }

    fn lister_icon() -> Option<Icon> {
        icon::from_file_data(include_bytes!("../../assets/icon.png"), None)
            .map_err(popup_error)
            .ok()
    }

    fn nav_bar(&'_ self) -> Element<'_, AppMessage> {
        row![
            button(text(tr!(&self.translations, "read_page")).align_x(Alignment::Center))
                .on_press(AppMessage::GoToRead)
                .style(match &self.current_page {
                    Page::Read(_) => button::primary,
                    Page::Write(_) => button::secondary,
                })
                .width(Length::Fill),
            button(text(tr!(&self.translations, "write_page")).align_x(Alignment::Center))
                .on_press(AppMessage::GoToWrite)
                .style(match &self.current_page {
                    Page::Read(_) => button::secondary,
                    Page::Write(_) => button::primary,
                })
                .width(Length::Fill)
        ]
        .spacing(10)
        .into()
    }

    fn language_toggle(&'_ self) -> Element<'_, AppMessage> {
        let label = match self.current_language {
            Language::English => "EN",
            Language::French => "FR",
        };

        let toggle_button = button(text(label))
            .on_press(AppMessage::ChangeLanguage(self.current_language.toggle()));

        row![Space::with_width(Length::Fill), toggle_button]
            .width(Length::Fill)
            .into()
    }

    fn change_language(&self, language: Language) -> Task<AppMessage> {
        let language_use_case = self.service.language_use_case.clone();
        Task::perform(
            async move {
                language_use_case.set_language(language.clone()).ok();
                let translations = language_use_case
                    .load_translations(&language)
                    .unwrap_or_default();
                (language, translations)
            },
            |(language, translations)| AppMessage::LanguageChanged(language, translations),
        )
    }
}
