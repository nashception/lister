use crate::domain::model::file_entry::FileWithMetadata;
use crate::domain::model::language::Language;
use crate::ui::messages::read_message::ReadMessage;
use crate::ui::utils::format_date_time::format_date_time;
use humansize::{format_size, DECIMAL};
use iced::widget::scrollable::{AbsoluteOffset, RelativeOffset};
use iced::widget::{column, operation, row, rule, text, Scrollable};
use iced::widget::Id;
use iced::{Element, Length};

pub struct FileList {
    pub files: Vec<FileWithMetadata>,
    pub scroll_bar_id: Id,
}

impl FileList {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            scroll_bar_id: Id::unique(),
        }
    }

    pub fn set_files(&mut self, files: Vec<FileWithMetadata>) {
        self.files = files;
    }

    pub fn clear(&mut self) {
        self.files.clear();
    }

    pub fn view<'a>(&'a self, language: &Language) -> Element<'a, ReadMessage> {
        let file_rows: Vec<Element<'a, ReadMessage>> = self
            .files
            .iter()
            .map(|file| {
                row![
                    text(&file.category_name).width(Length::FillPortion(1)),
                    text(&file.drive_name).width(Length::FillPortion(2)),
                    text(format_size(file.drive_available_space, DECIMAL))
                        .width(Length::FillPortion(1)),
                    text(format_date_time(file.drive_insertion_time, language))
                        .width(Length::FillPortion(2)),
                    text(file.parent_directory()).width(Length::FillPortion(3)),
                    text(file.filename()).width(Length::FillPortion(4)),
                    text(format_size(file.size_bytes, DECIMAL))
                        .width(Length::FillPortion(1))
                ]
                    .padding(3)
                    .into()
            })
            .collect();

        column![
            rule::horizontal(1),
            Scrollable::new(column(file_rows))
                .id(self.scroll_bar_id.clone())
                .height(Length::Fill),
            rule::horizontal(1),
        ]
            .into()
    }

    pub fn snap_to_top(&self) -> iced::Task<ReadMessage> {
        operation::snap_to(self.scroll_bar_id.clone(), RelativeOffset::START)
    }

    pub fn snap_to_bottom(&self) -> iced::Task<ReadMessage> {
        operation::snap_to(self.scroll_bar_id.clone(), RelativeOffset::END)
    }

    pub fn scroll(&self, dy: f32, shift: bool) -> iced::Task<ReadMessage> {
        let offset = if shift { dy * 33. } else { dy };
        operation::scroll_by(
            self.scroll_bar_id.clone(),
            AbsoluteOffset { x: 0.0, y: offset },
        )
    }
}