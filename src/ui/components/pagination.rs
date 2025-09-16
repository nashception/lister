use crate::ui::utils::translation::tr_impl;
use iced::widget::{button, row, text, text_input};
use iced::{Alignment, Element, Length};
use std::collections::HashMap;

use crate::tr;
use crate::ui::messages::read_message::ReadMessage;

pub struct Pagination {
    pub total_count: i64,
    pub current_page_index: usize,
    pub page_input_value: String,
    pub items_per_page: usize,
}

impl Pagination {
    pub fn new(items_per_page: usize) -> Self {
        Self {
            total_count: 0,
            current_page_index: 0,
            page_input_value: String::new(),
            items_per_page,
        }
    }

    pub fn total_pages(&self) -> usize {
        if self.total_count == 0 {
            1
        } else {
            ((self.total_count as usize) + self.items_per_page - 1) / self.items_per_page
        }
    }

    pub fn navigate_to(&mut self, page_index: usize) -> Option<usize> {
        if page_index < self.total_pages() {
            self.current_page_index = page_index;
            Some(page_index)
        } else {
            None
        }
    }

    pub fn first_page(&mut self) -> Option<usize> {
        self.navigate_to(0)
    }

    pub fn last_page(&mut self) -> Option<usize> {
        self.navigate_to(self.total_pages().saturating_sub(1))
    }

    pub fn next(&mut self) -> Option<usize> {
        if self.current_page_index + 1 < self.total_pages() {
            self.current_page_index += 1;
            Some(self.current_page_index)
        } else {
            None
        }
    }

    pub fn prev(&mut self) -> Option<usize> {
        if self.current_page_index > 0 {
            self.current_page_index -= 1;
            Some(self.current_page_index)
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.current_page_index = 0;
        self.page_input_value.clear();
    }

    pub fn clear(&mut self) {
        self.total_count = 0;
        self.reset();
    }

    pub fn view(&'_ self, translations: &HashMap<String, String>) -> Element<'_, ReadMessage> {
        let total_pages = self.total_pages();

        let first_button = button(text(tr!(translations, "first_button")))
            .on_press_maybe(if self.current_page_index > 0 {
                Some(ReadMessage::FirstPage)
            } else {
                None
            })
            .padding(8);

        let prev_button = button(text(tr!(translations, "prev_button")))
            .on_press_maybe(if self.current_page_index > 0 {
                Some(ReadMessage::PrevPage)
            } else {
                None
            })
            .padding(8);

        let page_info = text(format!(
            "{:^5} / {:^5} - {:^7}",
            if self.total_count == 0 {
                0
            } else {
                self.current_page_index + 1
            },
            if self.total_count == 0 {
                0
            } else {
                total_pages
            },
            self.total_count
        ))
        .size(14);

        let next_button = button(text(tr!(translations, "next_button")))
            .on_press_maybe(if self.current_page_index < total_pages.saturating_sub(1) {
                Some(ReadMessage::NextPage)
            } else {
                None
            })
            .padding(8);

        let last_button = button(text(tr!(translations, "last_button")))
            .on_press_maybe(if self.current_page_index < total_pages.saturating_sub(1) {
                Some(ReadMessage::LastPage)
            } else {
                None
            })
            .padding(8);

        let page_input = text_input(
            &tr!(translations, "page_placeholder"),
            &self.page_input_value,
        )
        .on_input(ReadMessage::PageInputChanged)
        .on_submit(ReadMessage::PageInputSubmit)
        .padding(8)
        .width(Length::Fixed(100f32));

        row![
            first_button,
            prev_button,
            page_info,
            next_button,
            last_button,
            page_input,
        ]
        .spacing(20)
        .align_y(Alignment::Center)
        .into()
    }
}
