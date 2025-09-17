use crate::tr;
use crate::ui::messages::write_message::WriteMessage;
use crate::ui::utils::translation::tr_impl;
use iced::widget::column;
use iced::widget::{button, text, Rule};
use iced::Element;
use std::collections::HashMap;

#[derive(PartialEq)]
pub enum IndexingState {
    Ready,
    CleaningDatabase,
    Scanning,
    Saving,
    Completed { files_indexed: usize },
}

pub fn indexing_state<'a>(
    state: &IndexingState,
    translations: &HashMap<String, String>,
) -> Element<'a, WriteMessage> {
    match state {
        IndexingState::Ready => column![].into(),
        IndexingState::CleaningDatabase => column![
            text(tr!(translations, "clean_status"))
                .size(18)
                .style(text::primary),
            text(tr!(translations, "clean_details"))
                .style(text::secondary)
                .size(14),
        ]
        .spacing(10)
        .into(),
        IndexingState::Scanning => column![
            text(tr!(translations, "scan_status"))
                .size(18)
                .style(text::primary),
            text(tr!(translations, "scan_details"))
                .style(text::secondary)
                .size(14),
        ]
        .spacing(10)
        .into(),
        IndexingState::Saving => column![
            text(tr!(translations, "save_status"))
                .size(18)
                .style(text::primary),
            text(tr!(translations, "save_details"))
                .style(text::secondary)
                .size(14),
        ]
        .spacing(10)
        .into(),
        IndexingState::Completed { files_indexed } => column![
            Rule::horizontal(1),
            iced::widget::column![
                text(tr!(translations, "done_status"))
                    .size(18)
                    .style(text::success),
                text(tr!(translations, "done_details", "nb_files" => &files_indexed.to_string()))
                    .style(text::success)
                    .size(14),
                button(text(tr!(translations, "start_new_indexing")))
                    .on_press(WriteMessage::ResetForm)
                    .padding(10)
                    .style(button::secondary),
            ]
            .spacing(10),
        ]
        .spacing(15)
        .into(),
    }
}
