use crate::domain::entities::pagination::PaginatedResult;

#[derive(Clone, Debug)]
pub enum ReadMessage {
    FirstPage,
    PrevPage,
    PageInputChanged(String),
    PageInputSubmit,
    NextPage,
    LastPage,
    DrivesFetched(Vec<String>),
    DriveSelected(String),
    SearchSubmit,
    ContentChanged(String),
    SearchClear,
    FilesLoaded(PaginatedResult),
    ArrowLeftPressed { shift: bool },
    ArrowRightPressed { shift: bool },
    ArrowUpPressed { shift: bool },
    ArrowDownPressed { shift: bool },
    ArrowNavigationReleased,
    PageUpPressed,
    PageDownPressed,
    HomePressed,
    EndPressed,
}
