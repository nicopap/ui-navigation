use bevy::ecs::entity::Entity;
use bevy::math::Vec2;
use non_empty_vec::NonEmpty;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NavRequest {
    Move(Direction),
    MenuMove(MenuDirection),
    Action,
    Cancel,
    FocusOn(Entity),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MenuDirection {
    Next,
    Previous,
}
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Direction {
    South,
    North,
    East,
    West,
}
impl Direction {
    pub(crate) fn is_in(&self, reference: Vec2, other: Vec2) -> bool {
        let coord = other - reference;
        use Direction::*;
        match self {
            South => coord.y < coord.x && coord.y < -coord.x,
            North => coord.y > coord.x && coord.y > -coord.x,
            East => coord.y < coord.x && coord.y > -coord.x,
            West => coord.y > coord.x && coord.y < -coord.x,
        }
    }
}

#[derive(Debug, Clone)]
pub enum NavEvent {
    /// Focus changed
    /// - `from`: the list of active elements from the focused one to the last
    ///   active which is affected by the focus change
    /// - `to`: the list of elements that has become active after the focus
    ///   change
    ///
    /// ## Notes
    /// Both lists are ascending, meaning that the focused and newly
    /// focused elements are the first of their respective vectors.
    ///
    /// [[NonEmpty]] enables you to safely check `to.first()` or `from.first()`
    /// without returning an option. It is guaranteed that there is at least
    /// one element.
    FocusChanged {
        to: NonEmpty<Entity>,
        from: NonEmpty<Entity>,
    },
    Caught {
        from: NonEmpty<Entity>,
        request: NavRequest,
    },
}
impl NavEvent {
    /// Convenience function to construct a `FocusChanged` with a single `to`
    ///
    /// Usually the `NavEvent::FocusChanged.to` field has a unique value.
    pub(crate) fn focus_changed(to: Entity, from: NonEmpty<Entity>) -> NavEvent {
        NavEvent::FocusChanged {
            from,
            to: NonEmpty::new(to),
        }
    }
}
