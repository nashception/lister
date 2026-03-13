use crate::domain::model::pagination::PaginatedResult;
use crate::ui::messages::drive_combo_box::DriveComboBoxMessage;

#[derive(Clone, Debug)]
pub enum ReadMessage {
    ArrowDownPressed { shift: bool },
    ArrowLeftPressed { shift: bool },
    ArrowNavigationReleased,
    ArrowRightPressed { shift: bool },
    ArrowUpPressed { shift: bool },
    ContentChanged(String),
    DriveComboBox(DriveComboBoxMessage),
    EndPressed,
    FilesLoaded(PaginatedResult),
    FirstPage,
    HomePressed,
    LastPage,
    NextPage,
    PageDownPressed,
    PageInputChanged(String),
    PageInputSubmit,
    PageUpPressed,
    PrevPage,
    SearchClear,
    SearchSubmit,
}
