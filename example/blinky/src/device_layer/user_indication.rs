use crate::device_layer::i_user_indication::IUi;

/// A concrete implementation of `IUi`.
pub struct ConsoleUi;

impl IUi for ConsoleUi {
    fn render(&mut self) {}
}