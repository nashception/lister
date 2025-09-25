use crate::domain::entities::language::Language;
use chrono::NaiveDateTime;

pub fn format_date_time(date_time: NaiveDateTime, language: &Language) -> String {
    date_time
        .format(match language {
            Language::English => "%Y-%m-%d %H:%M:%S",
            Language::French => "%d/%m/%Y %H:%M:%S",
        })
        .to_string()
}
