#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Command {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Previous,
    Next,
    Action,
    Cancel,
}

pub enum Event<T> {
    FocusChanged {
        to: T,
        disactivated: Vec<T>,
    },
    Caught {
        container: T,
        focused: T,
        command: Command,
    },
    Uncaught {
        focused: T,
        command: Command,
    },
}
