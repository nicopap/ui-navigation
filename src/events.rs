use bevy::ecs::entity::Entity;
use bevy::math::Vec2;
use non_empty_vec::NonEmpty;

/// Requests to send to the navigation system to update focus
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NavRequest {
    /// Move in 2d in provided direction
    Move(Direction),
    /// Move within the encompassing [`NavMenu::scope`](crate::NavMenu::scope)
    ScopeMove(ScopeDirection),
    /// Enter submenu if any [`NavMenu::reachable_from`](crate::NavMenu::reachable_from)
    /// the currently focused entity.
    Action,
    /// Leave this submenu to enter the one it is [`reachable_from`](crate::NavMenu::reachable_from)
    Cancel,
    /// Move the focus to any arbitrary [`Focusable`](crate::Focusable) entity
    FocusOn(Entity),
}

/// Direction for movement in [`NavMenu::scope`](crate::NavMenu::scope) menus.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ScopeDirection {
    Next,
    Previous,
}

/// 2d direction to move in normal menus
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
    pub(crate) fn opposite(&self) -> Self {
        use Direction::*;
        match self {
            South => North,
            East => West,
            West => East,
            North => South,
        }
    }
}

/// Events emitted by the navigation system.
///
/// Useful if you want to react to [`NavEvent::NoChanges`] event, for example
/// when a "start game" button is focused and the [`NavRequest::Action`] is
/// pressed.
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
    /// [`NonEmpty`] enables you to safely check `to.first()` or `from.first()`
    /// without returning an option. It is guaranteed that there is at least
    /// one element.
    FocusChanged {
        to: NonEmpty<Entity>,
        from: NonEmpty<Entity>,
    },
    /// The [`NavRequest`] didn't lead to any change in focus.
    NoChanges {
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
