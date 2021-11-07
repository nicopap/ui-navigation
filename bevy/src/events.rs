use bevy::ecs::entity::Entity;

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

#[derive(Debug)]
pub enum NavEvent {
    FocusChanged {
        to: Entity,
        disactivated: Vec<Entity>,
    },
    Caught {
        container: Entity,
        focused: Entity,
        command: Command,
    },
    Uncaught {
        focused: Entity,
        command: Command,
    },
}
