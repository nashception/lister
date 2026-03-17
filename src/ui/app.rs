use crate::domain::model::language::Language;
use crate::tr;
use crate::ui::app_factory::ListerAppService;
use crate::ui::messages::app_message::AppMessage;
use crate::ui::pages::delete_page::DeletePage;
use crate::ui::pages::read_page::ReadPage;
use crate::ui::pages::write_page::WritePage;
use crate::utils::dialogs::{popup_error, popup_info};
use humansize::{format_size, DECIMAL};
use iced::keyboard::key::Named;
use iced::keyboard::Modifiers;
use iced::widget::operation::{focus_next, focus_previous};
use iced::widget::{button, column, row, text, Space};
use iced::window::{icon, Icon, Settings};
use iced::{event, keyboard, Alignment, Element, Event, Length, Subscription, Task};
use std::collections::HashMap;

enum Page {
    Delete(DeletePage),
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
                Page::Delete(_) => DeletePage::title(&self.translations),
                Page::Read(_) => ReadPage::title(&self.translations),
                Page::Write(_) => WritePage::title(&self.translations),
            },
            env!("CARGO_PKG_VERSION")
        )
    }

    pub fn view(&'_ self) -> Element<'_, AppMessage> {
        let toolbar = self.toolbar();

        let nav_bar = self.nav_bar();

        let content = match &self.current_page {
            Page::Delete(page) => page.view(&self.translations).map(AppMessage::Delete),
            Page::Read(page) => page
                .view(&self.translations, &self.current_language)
                .map(AppMessage::Read),
            Page::Write(page) => page.view(&self.translations).map(AppMessage::Write),
        };

        column![toolbar, Space::new().height(10), nav_bar, content]
            .padding(20)
            .into()
    }

    pub fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::ChangeLanguage(language) => self.change_language(language),
            AppMessage::ChangePage => match self.current_page {
                Page::Delete(_) => self.update(AppMessage::GoToRead),
                Page::Read(_) => self.update(AppMessage::GoToWrite),
                Page::Write(_) => self.update(AppMessage::GoToDelete),
            },
            AppMessage::CompactDatabase => {
                let query_use_case = self.service.query_use_case.clone();
                Task::perform(
                    async move {
                        query_use_case.compact().unwrap_or_else(|err| {
                            popup_error(err);
                            0
                        })
                    },
                    AppMessage::DatabaseCompacted,
                )
            }
            AppMessage::DatabaseCompacted(freed_space) => {
                popup_info(
                    tr!(&self.translations, "compacted", "freed_space" => &format_size(freed_space, DECIMAL)),
                );
                Task::none()
            }
            AppMessage::Delete(msg) => {
                if let Page::Delete(page) = &mut self.current_page {
                    page.update(msg).map(AppMessage::Delete)
                } else {
                    Task::none()
                }
            }
            AppMessage::GoToDelete => {
                if matches!(self.current_page, Page::Delete(_)) {
                    Task::none()
                } else {
                    let (delete_page, task) = DeletePage::new(
                        self.service.delete_use_case.clone(),
                        self.service.query_use_case.clone(),
                    );
                    self.current_page = Page::Delete(delete_page);
                    task.map(AppMessage::Delete)
                }
            }
            AppMessage::GoToRead => {
                if matches!(self.current_page, Page::Read(_)) {
                    Task::none()
                } else {
                    let (read_page, task) = ReadPage::new(self.service.query_use_case.clone());
                    self.current_page = Page::Read(read_page);
                    task.map(AppMessage::Read)
                }
            }
            AppMessage::GoToWrite => {
                if matches!(self.current_page, Page::Write(_)) {
                    Task::none()
                } else {
                    let (write_page, task) = WritePage::new(
                        self.service.indexing_use_case.clone(),
                        self.service.directory_picker.clone(),
                    );
                    self.current_page = Page::Write(write_page);
                    task.map(AppMessage::Write)
                }
            }
            AppMessage::LanguageChanged(language, translations) => {
                self.current_language = language;
                self.translations = translations;
                Task::none()
            }
            AppMessage::Read(msg) => {
                if let Page::Read(page) = &mut self.current_page {
                    page.update(msg).map(AppMessage::Read)
                } else {
                    Task::none()
                }
            }
            AppMessage::TabPressed { shift } => {
                if shift {
                    focus_previous()
                } else {
                    focus_next()
                }
            }
            AppMessage::Write(msg) => {
                if let Page::Write(page) = &mut self.current_page {
                    page.update(msg).map(AppMessage::Write)
                } else {
                    Task::none()
                }
            }
        }
    }

    pub fn subscription(&self) -> Subscription<AppMessage> {
        let app_subscription = event::listen_with(|event, _status, _window| match event {
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
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
            }
            _ => None,
        });

        let page_subscription = match &self.current_page {
            Page::Delete(_) | Page::Write(_) => Subscription::none(),
            Page::Read(_) => ReadPage::subscription().map(AppMessage::Read),
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
                .style(if matches!(&self.current_page, Page::Read(_)) {
                    button::primary
                } else {
                    button::secondary
                })
                .width(Length::Fill),
            button(text(tr!(&self.translations, "write_page")).align_x(Alignment::Center))
                .on_press(AppMessage::GoToWrite)
                .style(if matches!(&self.current_page, Page::Write(_)) {
                    button::primary
                } else {
                    button::secondary
                })
                .width(Length::Fill),
            button(text(tr!(&self.translations, "delete_page")).align_x(Alignment::Center))
                .on_press(AppMessage::GoToDelete)
                .style(if matches!(&self.current_page, Page::Delete(_)) {
                    button::primary
                } else {
                    button::secondary
                })
                .width(Length::Fill)
        ]
        .spacing(10)
        .into()
    }

    fn toolbar(&'_ self) -> Element<'_, AppMessage> {
        row![
            Space::new().width(Length::Fill),
            button(text(tr!(&self.translations, "compact"))).on_press(AppMessage::CompactDatabase),
            button(text(self.current_language.to_string()))
                .on_press(AppMessage::ChangeLanguage(self.current_language.toggle()))
        ]
        .spacing(5)
        .width(Length::Fill)
        .into()
    }

    fn change_language(&self, language: Language) -> Task<AppMessage> {
        let language_use_case = self.service.language_use_case.clone();
        Task::perform(
            async move {
                language_use_case.set_language(&language).ok();
                let translations = language_use_case
                    .load_translations(&language)
                    .unwrap_or_default();
                (language, translations)
            },
            |(language, translations)| AppMessage::LanguageChanged(language, translations),
        )
    }
}
