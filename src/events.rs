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
    /// Unlocks the navigation system.
    ///
    /// A [`NavEvent::Unlocked`] will be emitted
    Free,
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
}

/// Events emitted by the navigation system.
///
/// Useful if you want to react to [`NavEvent::NoChanges`] event, for example
/// when a "start game" button is focused and the [`NavRequest::Action`] is
/// pressed.
#[derive(Debug, Clone)]
pub enum NavEvent {
    /// Focus changed
    ///
    /// ## Notes
    ///
    /// Both `to` and `from` are ascending, meaning that the focused and newly
    /// focused elements are the first of their respective vectors.
    ///
    /// [`NonEmpty`] enables you to safely check `to.first()` or `from.first()`
    /// without returning an option. It is guaranteed that there is at least
    /// one element.
    FocusChanged {
        /// The list of elements that has become active after the focus
        /// change
        to: NonEmpty<Entity>,
        /// The list of active elements from the focused one to the last
        /// active which is affected by the focus change
        from: NonEmpty<Entity>,
    },
    /// The [`NavRequest`] didn't lead to any change in focus.
    NoChanges {
        /// The list of active elements from the focused one to the last
        /// active which is affected by the focus change
        from: NonEmpty<Entity>,
        /// The [`NavRequest`] that didn't do anything
        request: NavRequest,
    },
    /// A [lock focusable](crate::Focusable::lock) has been triggered
    ///
    /// Once the navigation plugin enters a locked state, the only way to exit
    /// it is to send a [`NavRequest::Unlock`].
    Locked(Entity),
    /// A [lock focusable](crate::Focusable::lock) has been triggered
    ///
    /// Once the navigation plugin enters a locked state, the only way to exit
    /// it is to send a [`NavRequest::Unlock`].
    Unlocked(Entity),
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
