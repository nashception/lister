use crate::domain::model::language::Language;
use crate::ui::app::PageKind;
use crate::ui::messages::delete_message::DeleteMessage;
use crate::ui::messages::read_message::ReadMessage;
use crate::ui::messages::toaster_message::ToasterMessage;
use crate::ui::messages::write_message::WriteMessage;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum AppMessage {
    ChangeLanguage(Language),
    ChangePage(PageKind),
    ChangePageNext,
    CompactDatabase,
    DatabaseCompacted(u64),
    Delete(DeleteMessage),
    LanguageChanged(Language, HashMap<String, String>),
    Read(ReadMessage),
    TabPressed { shift: bool },
    ToastMessage(ToasterMessage),
    Write(WriteMessage),
}
