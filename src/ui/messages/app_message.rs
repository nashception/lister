use crate::domain::entities::language::Language;
use crate::ui::messages::read_message::ReadMessage;
use crate::ui::messages::write_message::WriteMessage;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum AppMessage {
    ChangeLanguage(Language),
    LanguageChanged(Language, HashMap<String, String>),
    GoToRead,
    GoToWrite,
    Read(ReadMessage),
    Write(WriteMessage),
    TabPressed { shift: bool },
    ChangePage,
}
