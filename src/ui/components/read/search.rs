use iced::widget::{button, column, focus_next, row, text, text_input};
use iced::{Element, Length, Task};
use std::collections::HashMap;

use crate::tr;
use crate::ui::messages::read_message::ReadMessage;

pub struct Search {
    pub query: String,
}

impl Search {
    pub fn new() -> (Self, Task<ReadMessage>) {
        (
            Self {
                query: String::new(),
            },
            focus_next(),
        )
    }

    pub fn clear(&mut self) {
        self.query.clear();
    }

    pub fn view(&'_ self, translations: &HashMap<String, String>) -> Element<'_, ReadMessage> {
        let search_input = text_input(&tr!(translations, "search_placeholder"), &self.query)
            .on_input(ReadMessage::ContentChanged)
            .on_submit(ReadMessage::SearchSubmit)
            .padding(10)
            .width(Length::Fill);

        let search_button = button(text(tr!(translations, "search_button")))
            .on_press(ReadMessage::SearchSubmit)
            .padding(10);

        let clear_button = button(text(tr!(translations, "clear_button")))
            .on_press(ReadMessage::SearchClear)
            .padding(10);

        column![row![search_input, search_button, clear_button].spacing(10)].into()
    }
}
