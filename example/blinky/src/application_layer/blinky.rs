use crate::device_layer::i_user_indication::IUi;

/// Business logic depending on `IUi` only through the generic bound.
pub struct Blinky<UiG: IUi> {
    ui: UiG,
}

impl<UiG: IUi> Blinky<UiG> {
    pub fn new(ui: UiG) -> Self {
        Self { ui }
    }

    pub fn run(&mut self) {
        self.ui.render();
    }
}