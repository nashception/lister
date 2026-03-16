use crate::domain::model::language::Language;
use crate::ui::messages::read_message::ReadMessage;
use crate::ui::messages::write_message::WriteMessage;
use std::collections::HashMap;
use crate::ui::messages::delete_message::DeleteMessage;

#[derive(Clone, Debug)]
pub enum AppMessage {
    ChangeLanguage(Language),
    ChangePage,
    CompactDatabase,
    DatabaseCompacted(u64),
    Delete(DeleteMessage),
    LanguageChanged(Language, HashMap<String, String>),
    GoToDelete,
    GoToRead,
    GoToWrite,
    Read(ReadMessage),
    TabPressed { shift: bool },
    Write(WriteMessage),
}
