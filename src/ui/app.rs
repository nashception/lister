use crate::domain::entities::language::Language;
use crate::domain::ports::primary::file_indexing_use_case::FileIndexingUseCase;
use crate::domain::ports::primary::file_query_use_case::FileQueryUseCase;
use crate::domain::ports::primary::language_use_case::LanguageManagementUseCase;
use crate::domain::ports::secondary::directory_picker::DirectoryPicker;
use crate::domain::services::file_indexing_service::FileIndexingService;
use crate::domain::services::file_query_service::FileQueryService;
use crate::domain::services::language_service::LanguageService;
use crate::infrastructure::database::sqlite_repository::SqliteFileRepository;
use crate::infrastructure::filesystem::native_directory_picker::NativeDirectoryPicker;
use crate::infrastructure::i18n::json_translation_loader::JsonTranslationLoader;
use crate::tr;
use crate::ui::messages::app_message::AppMessage;
use crate::ui::pages::read_page::ReadPage;
use crate::ui::pages::write_page::WritePage;
use crate::ui::utils::translation::tr_impl;
use crate::utils::dialogs::{popup_error, popup_error_and_exit};
use iced::keyboard::key::Named;
use iced::keyboard::Modifiers;
use iced::widget::{button, row, text, Row, Space};
use iced::window::{icon, Icon, Settings};
use iced::{keyboard, widget, Alignment, Element, Length, Subscription, Task};
use std::collections::HashMap;
use std::sync::Arc;

enum Page {
    Read(ReadPage),
    Write(WritePage),
}

pub struct ListerApp {
    query_use_case: Arc<dyn FileQueryUseCase>,
    indexing_use_case: Arc<dyn FileIndexingUseCase>,
    language_use_case: Arc<dyn LanguageManagementUseCase>,
    directory_picker: Arc<dyn DirectoryPicker>,
    current_language: Language,
    translations: HashMap<String, String>,
    current_page: Page,
}

impl ListerApp {
    pub fn new() -> (Self, Task<AppMessage>) {
        // Create the single repository instance
        let repository = Arc::new(
            SqliteFileRepository::new("app.db").unwrap_or_else(|error| popup_error_and_exit(error)),
        );
        let translation_loader = Arc::new(JsonTranslationLoader);
        let directory_picker = Arc::new(NativeDirectoryPicker);

        let query_service = Arc::new(FileQueryService::new(repository.clone()));
        let indexing_service = Arc::new(FileIndexingService::new(repository.clone()));
        let language_service =
            Arc::new(LanguageService::new(repository.clone(), translation_loader));

        let current_language = language_service
            .get_current_language()
            .unwrap_or_else(|_| Language::english());
        let translations = language_service
            .load_translations(&current_language)
            .unwrap_or_default();

        let (read_page, task) = ReadPage::new(query_service.clone());

        (
            Self {
                query_use_case: query_service,
                indexing_use_case: indexing_service,
                language_use_case: language_service,
                directory_picker,
                current_language,
                translations,
                current_page: Page::Read(read_page),
            },
            task.map(AppMessage::Read),
        )
    }

    pub fn window() -> Settings {
        Settings {
            icon: Self::lister_icon(),
            ..Default::default()
        }
    }

    pub fn title(&self) -> String {
        match &self.current_page {
            Page::Read(page) => page.title(&self.translations),
            Page::Write(page) => page.title(&self.translations),
        }
    }

    pub fn view(&'_ self) -> Element<'_, AppMessage> {
        let language_toggle = self.language_toggle();
        let nav_bar = self.nav_bar();

        let content = match &self.current_page {
            Page::Read(page) => page.view(&self.translations).map(AppMessage::Read),
            Page::Write(page) => page.view(&self.translations).map(AppMessage::Write),
        };

        iced::widget::column![language_toggle, Space::with_height(10), nav_bar, content]
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
                    let (read_page, task) = ReadPage::new(self.query_use_case.clone());
                    self.current_page = Page::Read(read_page);
                    task.map(AppMessage::Read)
                } else {
                    Task::none()
                }
            }
            AppMessage::GoToWrite => {
                if matches!(self.current_page, Page::Read(_)) {
                    let (write_page, task) = WritePage::new(
                        self.indexing_use_case.clone(),
                        self.directory_picker.clone(),
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
            Page::Read(read_page) => read_page.subscription().map(AppMessage::Read),
            Page::Write(_) => Subscription::none(),
        };

        Subscription::batch(vec![app_subscription, page_subscription])
    }

    fn lister_icon() -> Option<Icon> {
        icon::from_file_data(include_bytes!("../../assets/icon.png"), None)
            .map_err(|error| popup_error(error))
            .ok()
    }

    fn nav_bar(&'_ self) -> Row<'_, AppMessage> {
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
    }

    fn language_toggle(&'_ self) -> Row<'_, AppMessage> {
        let label = match self.current_language.code() {
            "fr" => "FR",
            _ => "EN",
        };

        let toggle_button = button(text(label))
            .on_press(AppMessage::ChangeLanguage(self.current_language.toggle()));

        row![Space::with_width(Length::Fill), toggle_button].width(Length::Fill)
    }

    fn change_language(&mut self, language: Language) -> Task<AppMessage> {
        let language_use_case = self.language_use_case.clone();
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
