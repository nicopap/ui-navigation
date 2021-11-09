use bevy::ecs::entity::Entity;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum NavRequest {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Previous,
    Next,
    Action,
    Cancel,
}

#[derive(Debug, Clone)]
pub enum NavEvent {
    FocusChanged {
        to: Entity,
        from: Vec<Entity>,
    },
    Caught {
        focused: Entity,
        request: NavRequest,
    },
    Uncaught {
        focused: Entity,
        request: NavRequest,
    },
}
