use crate::application_layer::blinky::Blinky;
use crate::device_layer::user_indication::ConsoleUi;

/// Wires the concrete `ConsoleUi` implementation together with the
/// `Blinky` application logic.
pub struct Manager {
    blinky: Blinky<ConsoleUi>,
}

impl Manager {
    pub fn new() -> Self {
        let ui = ConsoleUi;
        let blinky = Blinky::new(ui);

        Self { blinky }
    }

    pub fn run(&mut self) {
        self.blinky.run();
    }
}
