// TODO: review all uses of `.unwrap()`!
mod events;

use std::cmp::Ordering;
use std::fmt;

use bevy::ecs::system::{QuerySingleError, SystemParam};
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use non_empty_vec::NonEmpty;

pub use crate::events::{Direction, NavEvent, NavRequest};

#[derive(SystemParam)]
struct NavQueries<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static Parent>,
    focusables: Query<'w, 's, (Entity, &'static Focusable), With<Focusable>>,
    nav_fences: Query<'w, 's, (Entity, &'static NavFence), With<NavFence>>,
    transform: Query<'w, 's, &'static GlobalTransform>,
}

#[derive(Clone, Debug, Copy, PartialEq)]
enum FocusState {
    Dormant,
    Focused,
    Active,
    Inert,
}

/// A "scope" that isolate children [[Focusable]]s from other focusables and
/// specify navigation method within itself.
///
/// A `NavFence` can be used to:
/// * Prevent navigation from one specific submenu to another
/// * Specify the loop directions of navigation (going left when focusing on a
///   leftmost [[Focusable]] may go to the rightmost `Focusable`)
/// * Specify "tabbed menus" (TODO: name TBD) such that pressing
///   [[NavRequest::Next]] or [[NavRequest::Previous]] in a [[Focusable]]
///   nested within this `NavFence` will navigate this menu.
/// * Specify _submenus_ and specify from where those submenus are reachable
///
/// # Important
///
/// There are two important invariants to keep in mind:
///
/// 1. There should be **no cycles in the navigation graph**, ie:
///    You must ensure this doesn't create a cycle. You shouldn't be able
///    to reach `NavFence` X from [[Focusable]] Y if there is a path from
///    `NavFence` X to `Focusable` Y.
/// 2. There must be **at least one child [[Focusable]]** in the ui graph for each
///    `NavFence` when sending a [[NavRequest]]
#[derive(Debug, Component, Clone)]
pub struct NavFence {
    /// The `Focusable` of the scoping `NavFence` that links to this
    /// `NavFence` (None if this `NavFence` is the navigation graph root)
    focus_parent: Option<Entity>,
    // TODO:
    // The child of interest
    //
    // This is a sort of cache to not have to walk down the ECS hierarchy
    // every time we need to find the relevant child.
    // non_inert_child: CacheOption<Entity>,
}
impl NavFence {
    /// Prefer [[NavFence::reachable_from]] and [[NavFence::root]] to this
    ///
    /// `new` is useful to programmatically set the parent if you have an
    /// optional value. This saves you from a `match focus_parent`.
    pub fn new(focus_parent: Option<Entity>) -> Self {
        NavFence { focus_parent }
    }

    /// Set this fence as having no parents
    pub fn root() -> Self {
        NavFence { focus_parent: None }
    }

    // Should this be `unsafe`? Kinda annoying to have such an important part
    // of the API behind `unsafe`.
    /// Set this fence as reachable from a given [[Focusable]]
    ///
    /// When requesting [[NavRequest::Action]] when `focusable` is focused, the
    /// focus will be changed to a focusable within this fence.
    ///
    /// # Important
    ///
    /// You must ensure this doesn't create a cycle. Eg: you shouldn't be able
    /// to reach `NavFence` X from `Focusable` Y if there is a path from
    /// `NavFence` X to `Focusable` Y.
    pub fn reachable_from(focusable: Entity) -> Self {
        NavFence {
            focus_parent: Some(focusable),
        }
    }
}

/// An [[Entity]] that can be navigated to using the ui navigation system.
#[derive(Component, Clone, Copy)]
pub struct Focusable {
    focus_state: FocusState,
}
impl Default for Focusable {
    fn default() -> Self {
        Focusable {
            focus_state: FocusState::Inert,
        }
    }
}
impl fmt::Debug for Focusable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "F({:?})", self.focus_state)
    }
}
impl Focusable {
    /// This `Focusable` is the unique _focused_ element
    ///
    /// All navigation requests start from it.
    ///
    /// To set an arbitrary [[Focusable]] to _focused_, you should send a
    /// [[NavRequest::Focus]] request.
    pub fn is_focused(&self) -> bool {
        self.focus_state == FocusState::Focused
    }

    /// This `Focusable` is active
    ///
    /// Meaning it is either the _focused_ element or one of the `Focusable`
    /// on the path to it. All `Focusable`s to the path to the _focused_
    /// element are _active_.
    pub fn is_active(&self) -> bool {
        matches!(self.focus_state, FocusState::Active | FocusState::Focused)
    }

    /// This `Focusable` is dormant
    ///
    /// When focus leaves a specific `Focusable` without being acquired by a
    /// sibling, it becomes _dormant_. When focus comes back to the
    /// encompassing [[NavFence]], the _focused_ element will be the _dormant_
    /// element within the fence.
    pub fn is_dormant(&self) -> bool {
        self.focus_state == FocusState::Dormant
    }

    /// This `Focusable` is neither _active_, _focused_ or _dormant_
    pub fn is_inert(&self) -> bool {
        self.focus_state == FocusState::Inert
    }

    /// Programmatically create a `Focusable` with the given state.
    const fn with_state(focus_state: FocusState) -> Self {
        Focusable { focus_state }
    }
}

/// The currently _focused_ [[Focusable]]
///
/// You cannot edit it or create new `Focused` component. To set an arbitrary
/// [[Focusable]] to _focused_, you should send a [[NavRequest::FocusOn]]
/// request.
///
/// This [[Component]] is useful if you need to query for the _currently
/// focused_ element using a `Query<Entity, With<Focused>>` [[SystemParam]] for
/// example.
///
/// You can also check if a [[Focusable]] is _focused_ using
/// [[Focusable::is_focused]].
#[derive(Component)]
#[non_exhaustive]
pub struct Focused;

/// Which `Entity` in `siblings` can be reached from `focused` in
/// `direction` given entities `transform` if any, otherwise `None`
fn resolve_2d(
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

/// Resolves `request` when there is no more containing [[NavFence]]
///
/// This can happen for two distinct reasons:
/// 1. The Request cannot be resolved, in this case it is `Uncaught`
/// 2. It is a flat no-headache [[Focusable]] setup, in which case, we make sure
///    there is actually no [[NavFence]]s.
fn rootless_resolve(
    focused: Entity,
    request: NavRequest,
    queries: &NavQueries,
    from: NonEmpty<Entity>,
) -> NavEvent {
    if !queries.nav_fences.is_empty() {
        // In case the user has specified AT LEAST one NavFence, we will act as
        // if there were no orphan Focusable (this may not be true, but it
        // would be very expensive to manage that case)
        return NavEvent::Uncaught { request, from };
    }
    // In the case the user doesn't specify ANY NavFence, it's the most
    // simple graph (ie: no graph, only a flat relationship between Focusables)
    if let NavRequest::Move(direction) = request {
        let siblings: Vec<Entity> = queries.focusables.iter().map(|tpl| tpl.0).collect();
        match resolve_2d(focused, direction, &siblings, &queries.transform) {
            Some(to) => NavEvent::focus_changed(to, from),
            None => NavEvent::Uncaught { request, from },
        }
    } else {
        NavEvent::Uncaught { request, from }
    }
}

/// Resolve `request` where the focused element is `focused`
fn resolve(
    focused: Entity,
    request: NavRequest,
    queries: &NavQueries,
    from: Vec<Entity>,
) -> NavEvent {
    use NavRequest::*;

    assert!(
        queries.focusables.get(focused).is_ok(),
        "The resolution algorithm MUST go from a focusable element"
    );
    assert!(
        !from.contains(&focused),
        "Navigation graph cycle detected! This panic has prevented a stack overflow, \
        please check usages of `NavFence::reachable_from`"
    );

    let mut from = (from, focused).into();

    let (parent, nav_fence) = match parent_nav_fence(focused, queries) {
        Some(entity) => entity,
        None => return rootless_resolve(focused, request, queries, from),
    };
    match request {
        Move(direction) => {
            let siblings = children_focusables(parent, queries);
            let resolved = resolve_2d(focused, direction, &siblings, &queries.transform);
            match (resolved, nav_fence.focus_parent) {
                (None, Some(focused)) => resolve(focused, request, queries, from.into()),
                (Some(to), _) => NavEvent::focus_changed(to, from),
                (None, None) => NavEvent::Uncaught { from, request },
            }
        }
        Cancel => match nav_fence.focus_parent {
            Some(to) => {
                from.push(to);
                NavEvent::focus_changed(to, from)
            }
            None => NavEvent::Uncaught { from, request },
        },
        Action => {
            let child_nav_fence = queries
                .nav_fences
                .iter()
                .find(|e| e.1.focus_parent == Some(focused));
            match child_nav_fence {
                None => NavEvent::Uncaught { from, request },
                Some((child_nav_fence, _)) => {
                    let to = children_focusables(child_nav_fence, queries);
                    let to = non_inert_within(&to, queries).unwrap();
                    let to = (*to, from.clone().into()).into();
                    NavEvent::FocusChanged { to, from }
                }
            }
        }
        Next | Previous => {
            todo!("Manage 'menu' events")
        }
        FocusOn(_new_to_focus) => {
            todo!(
                "Create a FocusChanged event with \
                    ascending and descending path between \
                    the currently focused element and new_to_focus"
            )
        }
    }
}

/// Listen to [[NavRequest]] and update the state of [[Focusable]] entities if
/// relevant.
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
            assert!(
                !matches!(err, QuerySingleError::MultipleEntities(_)),
                "Multiple entities with Focused component, this should not happen"
            );
            queries.focusables.iter().next().unwrap().0
        });
        let event = resolve(focused_id, *request, &queries, Vec::new());
        if let NavEvent::FocusChanged { to, from } = &event {
            let focused = Focusable::with_state(FocusState::Focused);
            let inert = Focusable::with_state(FocusState::Inert);
            let dormant = Focusable::with_state(FocusState::Dormant);
            let active = Focusable::with_state(FocusState::Active);

            let (disable, put_to_sleep) = from.split_last();
            commands.entity(*disable).insert(inert).remove::<Focused>();
            for entity in put_to_sleep {
                commands.entity(*entity).insert(dormant).remove::<Focused>();
            }

            let (focus, activate) = to.split_first();
            commands.entity(*focus).insert(focused).insert(Focused);
            for entity in activate {
                commands.entity(*entity).insert(active);
            }
        };
        events.send(event);
    }
}

/// The [[NavFence]] containing `focusable`, if any
fn parent_nav_fence(focusable: Entity, queries: &NavQueries) -> Option<(Entity, NavFence)> {
    let Parent(parent) = queries.parents.get(focusable).ok()?;
    match queries.nav_fences.get(*parent) {
        Ok(nav_fence) => Some((*parent, nav_fence.1.clone())),
        Err(_) => parent_nav_fence(*parent, queries),
    }
}

/// All sibling [[Focusable]]s within a single [[NavFence]]
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

/// Which `Entity` in `siblings` is not _inert_, or the first in `siblings` if
/// none found.
fn non_inert_within<'a, 'b>(siblings: &'a [Entity], queries: &'b NavQueries) -> Option<&'a Entity> {
    siblings
        .iter()
        .find(|e| queries.focusables.get(**e).iter().any(|f| !f.1.is_inert()))
        .or_else(|| siblings.first())
}

pub struct NavigationPlugin;
impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NavRequest>()
            .add_event::<NavEvent>()
            .add_system(listen_nav_requests);
    }
}
