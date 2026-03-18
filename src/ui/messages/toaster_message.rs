use iced_toaster::{Toast, ToastId};
use crate::ui::messages::app_message::AppMessage;

#[derive(Clone, Debug)]
pub enum ToasterMessage {
    PushToast(Toast<AppMessage>),
    DismissToast(ToastId),
    HoverToast(ToastId, bool),
    Tick,
}
