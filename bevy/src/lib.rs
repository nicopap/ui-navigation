// TODO: review all uses of `.unwrap()`!
mod events;

use std::cmp::Ordering;

use bevy::ecs::system::{QuerySingleError, SystemParam};
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;

pub use crate::events::{NavEvent, NavRequest};

#[derive(SystemParam)]
struct NavQueries<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static Parent>,
    focusables: Query<'w, 's, (Entity, &'static Focusable), With<Focusable>>,
    nav_fences: Query<'w, 's, &'static NavFence, With<NavFence>>,
    transform: Query<'w, 's, &'static GlobalTransform>,
}

#[derive(Component, Default)]
#[non_exhaustive]
pub struct NavFence;

#[derive(Component, Default)]
pub struct Focusable {
    is_active: bool,
    is_focused: bool,
}
impl Focusable {
    pub fn is_focused(&self) -> bool {
        self.is_focused
    }
    pub fn is_active(&self) -> bool {
        self.is_active
    }
    fn focused() -> Self {
        Focusable {
            is_focused: true,
            is_active: true,
        }
    }
}

// TODO: consider using bevy::prelude::Interaction
// or at least something more ergonomic you can use methods on
/// Currently prevent user from setting Focused and Active
///
/// We just assume the first Focusable is an `Active` (and by extension a
/// `Focused`, as currently there is no tree traversal) Then once we have a
/// `Focused` everything should work as expected.
#[derive(Component)]
#[non_exhaustive]
pub struct Focused;

// Alternative design: Instead of evaluating at focus-change time the
// neighbores, somehow cache them
enum Direction {
    South,
    North,
    East,
    West,
}
impl Direction {
    fn is_in(&self, reference: Vec2, other: Vec2) -> bool {
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
impl TryFrom<NavRequest> for Direction {
    type Error = ();
    fn try_from(value: NavRequest) -> Result<Self, Self::Error> {
        use Direction::*;
        use NavRequest::*;
        match value {
            MoveUp => Ok(North),
            MoveDown => Ok(South),
            MoveLeft => Ok(West),
            MoveRight => Ok(East),
            _ => Err(()),
        }
    }
}

/// Which `Entity` in `siblings` can be reached from `focused` in
/// `direction` given entities `transform` if any, otherwise `None`
fn resolve_location_within(
    focused: Entity,
    direction: Direction,
    siblings: &[Entity],
    transform: &Query<&GlobalTransform>,
) -> Option<Entity> {
    let focused_loc = transform.get(focused).unwrap().translation.xy();
    siblings
        .iter()
        .filter(|sibling| {
            let sibling_loc = transform.get(**sibling).unwrap().translation;
            direction.is_in(focused_loc, sibling_loc.xy()) && **sibling != focused
        })
        .min_by(|s1, s2| {
            let s1_loc = transform.get(**s1).unwrap().translation;
            let s2_loc = transform.get(**s2).unwrap().translation;
            let s1_dist = focused_loc.distance_squared(s1_loc.xy());
            let s2_dist = focused_loc.distance_squared(s2_loc.xy());
            s1_dist.partial_cmp(&s2_dist).unwrap_or(Ordering::Equal)
        })
        .cloned()
}

/// Change focus within provided set of `siblings`, `None` if impossible.
fn resolve_within(
    focused: Entity,
    request: NavRequest,
    siblings: &[Entity],
    transform: &Query<&GlobalTransform>,
) -> Option<Entity> {
    let direction: Direction = request.try_into().ok()?;
    resolve_location_within(focused, direction, siblings, transform)
}

/// Resolves `request` when there is no more containing `NavFence`
///
/// This can happen for two distinct reasons:
/// 1. The Request cannot be resolved, in ths case it is `Uncaught`
/// 2. It is a flat no-headache `Focusable` setup, in which case, we make sure
///    there is actually no `NavFence`s.
fn rootless_resolve(
    focused: Entity,
    request: NavRequest,
    queries: &NavQueries,
    mut from: Vec<Entity>,
) -> NavEvent {
    // In the case the user doesn't specify ANY NavFence, it's the most
    // simple case
    if queries.nav_fences.is_empty() {
        let siblings: Vec<Entity> = queries.focusables.iter().map(|tpl| tpl.0).collect();
        from.push(focused);
        match resolve_within(focused, request, &siblings, &queries.transform) {
            Some(to) => NavEvent::FocusChanged { to, from },
            None => NavEvent::Uncaught { request, focused },
        }
    } else {
        // In case the user has specified AT LEAST one NavFence, we will act as
        // if there were no orphan Focusable (this may not be true, but it
        // would be very expensive to manage that case)
        let focused = *from.last().unwrap();
        NavEvent::Uncaught { request, focused }
    }
}

/// Resolve `request` where the focused element is `focused`
fn resolve(
    focused: Entity,
    request: NavRequest,
    queries: &NavQueries,
    mut from: Vec<Entity>,
) -> NavEvent {
    let nav_fence = match parent_nav_fence(focused, queries) {
        Some(entity) => entity,
        None => return rootless_resolve(focused, request, queries, from),
    };
    let siblings = children_focusables(nav_fence, queries);
    let focused = get_active(&siblings, queries).unwrap();
    from.push(focused);
    match resolve_within(focused, request, &siblings, &queries.transform) {
        Some(to) => NavEvent::FocusChanged { to, from },
        None => resolve(nav_fence, request, queries, from),
    }
}
// Consideration: exploring a part of the UI graph every time a navigation
// request is sent might be too slow. Possible mitigation: caching parts of the
// navigation graph.
fn listen_nav_requests(
    focused: Query<Entity, With<Focused>>,
    mut requests: EventReader<NavRequest>,
    queries: NavQueries,
    mut events: EventWriter<NavEvent>,
    mut commands: Commands,
) {
    // TODO: this most likely breaks when there is more than a single event
    for request in requests.iter() {
        // TODO: This code needs cleanup
        let focused_id = focused.get_single().unwrap_or_else(|err| {
            if matches!(err, QuerySingleError::MultipleEntities(_)) {
                panic!("Multiple focused, not possible");
            }
            queries.focusables.iter().next().unwrap().0
        });
        let event = resolve(focused_id, *request, &queries, Vec::new());
        if let NavEvent::FocusChanged { to, from } = event.clone() {
            for elem in from {
                commands.entity(elem).remove::<Focused>();
                commands.entity(elem).insert(Focusable::default());
            }
            commands
                .entity(to)
                .insert_bundle((Focused, Focusable::focused()));
        };
        events.send(event);
    }
}

/// The `NavFence` containing `focusable`, if any
fn parent_nav_fence(focusable: Entity, queries: &NavQueries) -> Option<Entity> {
    match queries.parents.get(focusable).ok() {
        Some(Parent(parent)) if queries.nav_fences.get(*parent).is_ok() => Some(*parent),
        Some(Parent(parent)) => parent_nav_fence(*parent, queries),
        None => None,
    }
}

/// All sibling focusables within a single NavFence
fn children_focusables(nav_fence: Entity, queries: &NavQueries) -> Vec<Entity> {
    match queries.children.get(nav_fence).ok() {
        Some(direct_children) => {
            let focusables = direct_children
                .iter()
                .filter(|e| queries.focusables.get(**e).is_ok())
                .cloned();
            let transitive_focusables = direct_children
                .iter()
                .filter(|e| queries.focusables.get(**e).is_err())
                .filter(|e| queries.nav_fences.get(**e).is_err())
                .flat_map(|e| children_focusables(*e, queries));
            focusables.chain(transitive_focusables).collect()
        }
        None => Vec::new(),
    }
}

/// Which `Entity` in `focusables` is `Active`, or the first in `focusables` if
/// none found.
///
/// (which is shouldn't happen outside of the very first focus action)
fn get_active(focusables: &[Entity], queries: &NavQueries) -> Option<Entity> {
    focusables
        .iter()
        .find(|e| queries.focusables.get(**e).iter().any(|f| f.1.is_active))
        .or_else(|| focusables.first())
        .cloned()
}

pub struct NavigationPlugin;
impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NavRequest>()
            .add_event::<NavEvent>()
            .add_system(listen_nav_requests);
    }
}
