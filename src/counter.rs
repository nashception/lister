use iced::widget::{button, text, Column};

// fn main() -> iced::Result {
//     iced::run("A cool counter", Counter::update, Counter::view)
// }

#[derive(Default)]
struct Counter {
    value: i64,
}

#[derive(Clone, Debug)]
enum Message {
    Increment,
    Decrement,
}

impl Counter {
    fn view(&'_ self) -> Column<'_, Message> {
        iced::widget::column![
            button("+").on_press(Message::Increment),
            text(self.value),
            button("-").on_press(Message::Decrement),
        ]
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Increment => {
                self.value += 1;
            }
            Message::Decrement => {
                self.value -= 1;
            }
        }
    }
}

#[test]
fn it_counts_properly() {
    let mut counter = Counter::default();

    counter.update(Message::Increment);
    counter.update(Message::Increment);
    counter.update(Message::Decrement);

    assert_eq!(counter.value, 1);
}