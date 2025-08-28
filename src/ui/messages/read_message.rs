use crate::domain::entities::pagination::PaginatedResult;

#[derive(Clone, Debug)]
pub enum ReadMessage {
    FirstPage,
    PrevPage,
    PageInputChanged(String),
    PageInputSubmit,
    NextPage,
    LastPage,
    SearchSubmit,
    ContentChanged(String),
    SearchClear,
    FilesLoaded {
        task_id: u64,
        result: PaginatedResult,
    },
    ArrowLeftPressed {
        shift: bool,
    },
    ArrowRightPressed {
        shift: bool,
    },
    ArrowUpPressed {
        shift: bool,
    },
    ArrowDownPressed {
        shift: bool,
    },
}
