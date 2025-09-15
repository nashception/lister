use crate::domain::entities::file_entry::FileWithMetadata;
use crate::ui::messages::read_message::ReadMessage;
use humansize::{format_size, DECIMAL};
use iced::widget::{column, row, scrollable, text, Rule};
use iced::{Element, Length};

pub struct FileList {
    pub files: Vec<FileWithMetadata>,
    pub scroll_bar_id: scrollable::Id,
}

impl FileList {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            scroll_bar_id: scrollable::Id::unique(),
        }
    }

    pub fn set_files(&mut self, files: Vec<FileWithMetadata>) {
        self.files = files;
    }

    pub fn clear(&mut self) {
        self.files.clear();
    }

    pub fn view<'a>(&'a self) -> Element<'a, ReadMessage> {
        let file_rows: Vec<Element<'a, ReadMessage>> = self
            .files
            .iter()
            .map(|file| {
                row![
                    text(&file.category_name).width(Length::FillPortion(1)),
                    text(&file.drive_name).width(Length::FillPortion(2)),
                    text(format_size(file.drive_available_space as u64, DECIMAL))
                        .width(Length::FillPortion(1)),
                    text(file.parent_directory()).width(Length::FillPortion(3)),
                    text(file.filename()).width(Length::FillPortion(4)),
                    text(format_size(file.size_bytes as u64, DECIMAL))
                        .width(Length::FillPortion(1))
                ]
                .padding(3)
                .into()
            })
            .collect();

        column![
            Rule::horizontal(1),
            scrollable(column(file_rows))
                .id(self.scroll_bar_id.clone())
                .height(Length::Fill),
            Rule::horizontal(1),
        ]
        .into()
    }

    pub fn snap_to_top(&self) -> iced::Task<ReadMessage> {
        scrollable::snap_to(
            self.scroll_bar_id.clone(),
            scrollable::RelativeOffset::START,
        )
    }

    pub fn snap_to_bottom(&self) -> iced::Task<ReadMessage> {
        scrollable::snap_to(self.scroll_bar_id.clone(), scrollable::RelativeOffset::END)
    }

    pub fn scroll(&self, dy: f32, shift: bool) -> iced::Task<ReadMessage> {
        let offset = if shift { dy * 33. } else { dy };
        scrollable::scroll_by(
            self.scroll_bar_id.clone(),
            scrollable::AbsoluteOffset { x: 0.0, y: offset },
        )
    }
}
