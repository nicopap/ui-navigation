//! The resolution algorithm for the navigation system.
//!
//! # Overview
//!
//! This module defines two systems:
//! 1. [`listen_nav_requests`]: the system gathering [`NavRequest`] and running
//!    the [`resolve`] algorithm on them, updating the [`Focusable`] states and
//!    sending [`NavEvent`] as result.
//! 2. [`insert_tree_menus`]: system responsible to convert seed bundles defined
//!    in [`crate::seeds`] into [`TreeMenu`], which is the component used by
//!    the resolution algorithm.
//!
//! The module also defines the [`Focusable`] component (also used in the
//! resolution algorithm) and its fields.
//!
//! The bulk of the resolution algorithm is implemented in [`resolve`],
//! delegating some abstract tasks to helper functions, of which:
//! * [`parent_menu`]
//! * [`children_focusables`]
//! * [`child_menu`]
//! * [`focus_deep`]
//! * [`root_path`]
//! * [`resolve_2d`]
//! * [`resolve_scope`]
use std::cmp::Ordering;
use std::fmt;
use std::num::NonZeroUsize;

use bevy::{ecs::system::SystemParam, log::warn, math::Vec3Swizzles, prelude::*};
use non_empty_vec::NonEmpty;

use crate::{
    commands::set_focus_state,
    events::{self, NavEvent, NavRequest},
    seeds::{self, NavMenu as MenuSetting},
};

#[allow(clippy::type_complexity)]
#[derive(SystemParam)]
pub(crate) struct NavQueries<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static Parent>,
    focusables: Query<'w, 's, (Entity, &'static mut Focusable), Without<TreeMenu>>,
    menus: Query<'w, 's, (Entity, &'static mut TreeMenu), Without<Focusable>>,
    is_menu: Query<'w, 's, Entity, Or<(With<TreeMenu>, With<seeds::TreeMenuSeed>)>>,
    transform: Query<'w, 's, &'static GlobalTransform>,
}
impl<'w, 's> NavQueries<'w, 's> {
    fn focused(&self) -> Option<Entity> {
        use FocusState::{Dormant, Focused};
        let menu_dormant = |menu: &TreeMenu| menu.focus_parent.is_none().then(|| menu.active_child);
        let any_dormant = |(e, focus): (Entity, &Focusable)| (focus.state == Dormant).then(|| e);
        let any_dormant = || self.focusables.iter().find_map(any_dormant);
        let root_dormant = || self.menus.iter().find_map(|(_, menu)| menu_dormant(menu));
        let fallback = || self.focusables.iter().next().map(|(entity, _)| entity);
        self.focusables
            .iter()
            .find_map(|(e, focus)| (focus.state == Focused).then(|| e))
            .or_else(root_dormant)
            .or_else(any_dormant)
            .or_else(fallback)
    }
    fn set_entity_focus(&mut self, cmds: &mut Commands, entity: Entity, state: FocusState) {
        if let Ok((_, mut focusable)) = self.focusables.get_mut(entity) {
            focusable.state = state;
            cmds.add(set_focus_state(entity, state));
        }
    }
}

/// State of a [`Focusable`].
#[derive(Clone, Debug, Copy, PartialEq)]
pub enum FocusState {
    /// An entity that was previously [`Active`](FocusState::Active) from a branch of
    /// the menu tree that is currently not _focused_. When focus comes back to
    /// the [`NavMenu`](MenuSetting) containing this [`Focusable`], the `Dormant` element
    /// will be the [`Focused`](FocusState::Focused) entity.
    Dormant,
    /// The currently highlighted/used entity, there is only a signle _focused_
    /// entity.
    ///
    /// All navigation requests start from it.
    ///
    /// To set an arbitrary [`Focusable`] to _focused_, you should send a
    /// [`NavRequest::FocusOn`] request.
    Focused,
    /// This [`Focusable`] is on the path in the menu tree to the current
    /// [`Focused`](FocusState::Focused) entity.
    ///
    /// [`FocusState::Active`] focusables are the [`Focusable`]s from
    /// previous menus that were activated in order to reach the
    /// [`NavMenu`](MenuSetting) containing the currently _focused_ element.
    Active,
    /// None of the above: This [`Focusable`] is neither `Dormant`, `Focused`
    /// or `Active`.
    Inert,
}

/// The navigation system's lock.
///
/// When locked, the navigation system doesn't process any [`NavRequest`].
/// It only waits on a [`NavRequest::Free`] event. It will then continue
/// processing new requests.
pub struct NavLock {
    entity: Option<Entity>,
}
impl NavLock {
    pub(crate) fn new() -> Self {
        Self { entity: None }
    }
    /// The [`Entity`](https://docs.rs/bevy/0.7.0/bevy/ecs/entity/struct.Entity.html)
    /// that triggered the lock.
    pub fn entity(&self) -> Option<Entity> {
        self.entity
    }
    /// Whether the navigation system is locked.
    pub fn is_locked(&self) -> bool {
        self.entity.is_some()
    }
}

/// A menu that isolate children [`Focusable`]s from other focusables and
/// specify navigation method within itself.
///
/// The user can't create a `TreeMenu`, they will use the
/// [`NavMenu`](MenuSetting) API and the `TreeMenu` component will be inserted
/// by the [`insert_tree_menus`] system.
#[derive(Debug, Component, Clone)]
pub(crate) struct TreeMenu {
    /// The [`Focusable`] that sends to this `NavMenu` when recieving
    /// [`NavRequest::Action`].
    pub(crate) focus_parent: Option<Entity>,
    /// How we want the user to move between [`Focusable`]s within this menu.
    pub(crate) setting: MenuSetting,
    /// The currently dormant or active focusable in this menu.
    pub(crate) active_child: Entity,
}

/// The actions triggered by a [`Focusable`].
#[derive(Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum FocusAction {
    /// Acts like a standard navigation node.
    ///
    /// Goes into relevant menu if any [`NavMenu`](MenuSetting) is
    /// [`reachable_from`](MenuSetting::reachable_from) this [`Focusable`].
    Normal,

    /// If we receive [`NavRequest::Action`] while this [`Focusable`] is
    /// focused, it will act as a [`NavRequest::Cancel`] (leaving submenu to
    /// enter the parent one).
    Cancel,

    /// If we receive [`NavRequest::Action`] while this [`Focusable`] is
    /// focused, the navigation system will freeze until [`NavRequest::Free`]
    /// is received, sending a [`NavEvent::Unlocked`].
    ///
    /// This is useful to implement widgets with complex controls you don't
    /// want to accidentally unfocus, or suspending the navigation system while
    /// in-game.
    Lock,
}

/// An [`Entity`](https://docs.rs/bevy/0.7.0/bevy/ecs/entity/struct.Entity.html)
/// that can be navigated to using the ui navigation system.
///
/// It is in one of multiple [`FocusState`], you can check its state with
/// the [`Focusable::state`] method.
///
/// A `Focusable` can execute a variety of [`FocusAction`] when receiving
/// [`NavRequest::Action`], the default one is [`FocusAction::Normal`]
#[derive(Component, Clone)]
pub struct Focusable {
    pub(crate) state: FocusState,
    action: FocusAction,
}
impl Default for Focusable {
    fn default() -> Self {
        Focusable {
            state: FocusState::Inert,
            action: FocusAction::Normal,
        }
    }
}
impl fmt::Debug for Focusable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "F({:?})", self.state)
    }
}
impl Focusable {
    /// Default Focusable
    pub fn new() -> Self {
        Self::default()
    }
    /// The [`FocusState`] of this `Focusable`.
    pub fn state(&self) -> FocusState {
        self.state
    }
    /// The [`FocusAction`] of this `Focusable`.
    pub fn action(&self) -> FocusAction {
        self.action
    }
    /// Spawn a "cancel" focusable, see [`FocusAction::Cancel`].
    pub fn cancel() -> Self {
        Focusable {
            state: FocusState::Inert,
            action: FocusAction::Cancel,
        }
    }
    /// Spawn a "lock" focusable, see [`FocusAction::Lock`].
    pub fn lock() -> Self {
        Focusable {
            state: FocusState::Inert,
            action: FocusAction::Lock,
        }
    }
    /// Spawn a focusable that will get highlighted in priority when none are set yet.
    ///
    /// **WARNING**: Only use this to spawn the UI. Any of the following state is
    /// unspecified and will likely result in broken behavior:
    /// * Having multiple dormant `Focusable`s in the same menu.
    /// * Updating an already existing `Focusable` with this.
    pub fn dormant(self) -> Self {
        Self {
            state: FocusState::Dormant,
            ..self
        }
    }
}

/// The currently _focused_ [`Focusable`].
///
/// You cannot edit it or create new `Focused` component. To set an arbitrary
/// [`Focusable`] to _focused_, you should send [`NavRequest::FocusOn`].
///
/// This [`Component`](https://docs.rs/bevy/0.7.0/bevy/ecs/component/trait.Component.html)
/// is useful if you need to query for the _currently focused_ element using a
/// `Query<Entity, With<Focused>>` for example.
///
/// If a [`Focusable`] is focused, its [`Focusable::state()`] will be
/// [`FocusState::Focused`], if you have a [`Focusable`] but can't query
/// filter on [`Focused`], you can check for equality.
///
/// # Notes
///
/// The `Focused` marker component is only updated at the end of the
/// `CoreStage::Update` stage. This means it might lead to a single frame of
/// latency compared to using [`Focusable::state()`].
#[derive(Component)]
#[non_exhaustive]
pub struct Focused;

/// Which `Entity` in `siblings` can be reached from `focused` in
/// `direction` given entities `transform` if any, otherwise `None`.
fn resolve_2d<'a, 'b, 'c>(
    focused: Entity,
    direction: events::Direction,
    cycles: bool,
    siblings: &'a [Entity],
    transform: &'b Query<&'c GlobalTransform>,
) -> Option<&'a Entity> {
    use events::Direction::*;

    let pos_of = |entity: Entity| {
        transform
            .get(entity)
            .expect("Focusable entities must have a GlobalTransform component")
            .translation
            .xy()
    };
    let focused_pos = pos_of(focused);
    let closest = siblings
        .iter()
        .filter(|sibling| direction.is_in(focused_pos, pos_of(**sibling)) && **sibling != focused);
    let closest = max_by_in_iter(closest, |s| -focused_pos.distance_squared(pos_of(**s)));
    match closest {
        // Cycle if we do not find an entity in the requested direction
        // TODO: clean this up to handle properly camera offset and true screen size
        None if cycles => {
            let focused_pos = match direction {
                South => Vec2::new(focused_pos.x, 3000.0),
                North => Vec2::new(focused_pos.x, 0.0),
                East => Vec2::new(0.0, focused_pos.y),
                West => Vec2::new(3000.0, focused_pos.y),
            };
            max_by_in_iter(siblings.iter(), |s| {
                -focused_pos.distance_squared(pos_of(**s))
            })
        }
        anyelse => anyelse,
    }
}

pub(crate) fn max_by_in_iter<U, T: PartialOrd>(
    iter: impl Iterator<Item = U>,
    f: impl Fn(&U) -> T,
) -> Option<U> {
    iter.max_by(|s1, s2| {
        let s1_val = f(s1);
        let s2_val = f(s2);
        s1_val.partial_cmp(&s2_val).unwrap_or(Ordering::Equal)
    })
}

/// Returns the next or previous entity based on `direction`
fn resolve_scope(
    focused: Entity,
    direction: events::ScopeDirection,
    cycles: bool,
    siblings: &NonEmpty<Entity>,
) -> Option<&Entity> {
    let focused_index = siblings.iter().position(|e| *e == focused)?;
    let new_index = resolve_index(focused_index, cycles, direction, siblings.len().get() - 1);
    new_index.and_then(|i| siblings.get(i))
}

/// Resolve `request` where the focused element is `focused`
fn resolve(
    focused: Entity,
    request: NavRequest,
    queries: &NavQueries,
    lock: &mut NavLock,
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
        please check usages of `NavMenu::reachable_from`"
    );

    let mut from = (from, focused).into();

    macro_rules! or_none {
        ($to_match:expr) => {
            match $to_match {
                Some(x) => x,
                None => return NavEvent::NoChanges { from, request },
            }
        };
    }
    match request {
        Move(direction) => {
            let (parent, cycles) = match parent_menu(focused, queries) {
                Some(val) if !val.1.setting.is_2d() => {
                    return NavEvent::NoChanges { from, request }
                }
                Some(val) => (Some(val.0), !val.1.setting.bound()),
                None => (None, true),
            };
            let siblings = match parent {
                Some(parent) => children_focusables(parent, queries),
                None => {
                    let focusables: Vec<_> = queries.focusables.iter().map(|tpl| tpl.0).collect();
                    NonEmpty::try_from(focusables).expect(
                        "There must be at least one `Focusable` when sending a `NavRequest`!",
                    )
                }
            };
            let to = resolve_2d(focused, direction, cycles, &siblings, &queries.transform);
            NavEvent::focus_changed(*or_none!(to), from)
        }
        Cancel => {
            let to = or_none!(parent_menu(focused, queries));
            let to = or_none!(to.1.focus_parent);
            from.push(to);
            NavEvent::focus_changed(to, from)
        }
        Action => {
            if let Ok((_, focusable)) = queries.focusables.get(focused) {
                match focusable.action {
                    FocusAction::Cancel => {
                        let mut from = from.to_vec();
                        from.truncate(from.len() - 1);
                        return resolve(focused, NavRequest::Cancel, queries, lock, from);
                    }
                    FocusAction::Lock => {
                        lock.entity = Some(focused);
                        return NavEvent::Locked(focused);
                    }
                    FocusAction::Normal => {}
                }
            }
            let child_menu = child_menu(focused, queries);
            let (_, menu) = or_none!(child_menu);
            let to = (menu.active_child, from.clone().into()).into();
            NavEvent::FocusChanged { to, from }
        }
        ScopeMove(scope_dir) => {
            let (parent, menu) = or_none!(parent_menu(focused, queries));
            let siblings = children_focusables(parent, queries);
            if !menu.setting.is_scope() {
                let focused = or_none!(menu.focus_parent);
                resolve(focused, request, queries, lock, from.into())
            } else {
                let cycles = !menu.setting.bound();
                let to = or_none!(resolve_scope(focused, scope_dir, cycles, &siblings));
                let extra = match child_menu(*to, queries) {
                    Some((_, menu)) => focus_deep(menu, queries),
                    None => Vec::new(),
                };
                let to = (extra, *to).into();
                NavEvent::FocusChanged { to, from }
            }
        }
        FocusOn(new_to_focus) => {
            let mut from = root_path(focused, queries);
            let mut to = root_path(new_to_focus, queries);
            trim_common_tail(&mut from, &mut to);
            if from == to {
                NavEvent::NoChanges { from, request }
            } else {
                NavEvent::FocusChanged { from, to }
            }
        }
        Free => {
            if let Some(lock_entity) = lock.entity.take() {
                NavEvent::Unlocked(lock_entity)
            } else {
                warn!("Received a NavRequest::Free while not locked");
                NavEvent::NoChanges { from, request }
            }
        }
    }
}

/// Replaces [`seeds::TreeMenuSeed`]s with proper [`TreeMenu`]s.
pub(crate) fn insert_tree_menus(
    mut cmds: Commands,
    seeds: Query<(Entity, &seeds::TreeMenuSeed)>,
    queries: NavQueries,
) {
    use FocusState::{Active, Dormant, Focused};
    let mut inserts = Vec::new();
    for (entity, seed) in seeds.iter() {
        let children = children_focusables(entity, &queries);
        let child = children
            .iter()
            .find_map(|e| {
                let (_, focusable) = queries.focusables.get(*e).ok()?;
                matches!(focusable.state, Dormant | Active | Focused).then(|| e)
            })
            .unwrap_or_else(|| children.first());
        let menu = seed.clone().with_child(*child);
        inserts.push((entity, (menu,)));
        cmds.entity(entity).remove::<seeds::TreeMenuSeed>();
    }
    cmds.insert_or_spawn_batch(inserts)
}

/// System to set the first [`Focusable`] to [`FocusState::Focused`] when no
/// navigation has been done yet.
pub(crate) fn set_first_focused(
    has_focused: Query<(), With<Focused>>,
    mut queries: NavQueries,
    mut cmds: Commands,
    mut events: EventWriter<NavEvent>,
) {
    if has_focused.is_empty() {
        if let Some(to_focus) = queries.focused() {
            queries.set_entity_focus(&mut cmds, to_focus, FocusState::Focused);
            events.send(NavEvent::InitiallyFocused(to_focus));
        }
    }
}

/// Listen to [`NavRequest`] and update the state of [`Focusable`] entities if
/// relevant.
pub(crate) fn listen_nav_requests(
    mut cmds: Commands,
    mut queries: NavQueries,
    mut lock: ResMut<NavLock>,
    mut requests: EventReader<NavRequest>,
    mut events: EventWriter<NavEvent>,
) {
    use FocusState as Fs;

    let no_focused = "Tried to execute a NavRequest when no focusables exist, NavRequest does nothing if there isn't any navigation to do.";
    for request in requests.iter() {
        if lock.is_locked() && *request != NavRequest::Free {
            continue;
        }
        // TODO: ensure no multiple Focused entities
        let focused = if let Some(e) = queries.focused() {
            e
        } else {
            warn!(no_focused);
            continue;
        };
        let event = resolve(focused, *request, &queries, &mut lock, Vec::new());
        // Change focus state of relevant entities
        if let NavEvent::FocusChanged { to, from } = &event {
            if to == from {
                continue;
            }
            let (&disable, put_to_sleep) = from.split_last();
            queries.set_entity_focus(&mut cmds, disable, Fs::Inert);
            for &entity in put_to_sleep {
                queries.set_entity_focus(&mut cmds, entity, Fs::Dormant);
            }
            let (&focus, activate) = to.split_first();
            set_active_child(&mut cmds, focus, &mut queries);
            queries.set_entity_focus(&mut cmds, focus, Fs::Focused);
            for &entity in activate {
                set_active_child(&mut cmds, entity, &mut queries);
                queries.set_entity_focus(&mut cmds, entity, Fs::Active);
            }
        };
        events.send(event);
    }
}

/// Set the [`active_child`](TreeMenu::active_child) field of the enclosing
/// [`TreeMenu`] and disables the previous one.
fn set_active_child(cmds: &mut Commands, child: Entity, queries: &mut NavQueries) {
    let mut focusable = child;
    let mut nav_menu = loop {
        if let Ok(&Parent(parent)) = queries.parents.get(focusable) {
            focusable = parent;
            if let Ok(menu) = queries.menus.get_mut(parent) {
                break menu.1;
            }
        } else {
            return;
        }
    };
    let entity = nav_menu.active_child;
    nav_menu.active_child = child;
    queries.set_entity_focus(cmds, entity, FocusState::Inert);
}

/// The child [`TreeMenu`] of `focusable`.
fn child_menu<'a>(focusable: Entity, queries: &'a NavQueries) -> Option<(Entity, &'a TreeMenu)> {
    queries
        .menus
        .iter()
        .find(|e| e.1.focus_parent == Some(focusable))
}

/// The [`TreeMenu`] containing `focusable`, if any.
pub(crate) fn parent_menu(focusable: Entity, queries: &NavQueries) -> Option<(Entity, TreeMenu)> {
    let &Parent(parent) = queries.parents.get(focusable).ok()?;
    match queries.menus.get(parent) {
        Ok(menu) => Some((parent, menu.1.clone())),
        Err(_) => parent_menu(parent, queries),
    }
}

/// All sibling [`Focusable`]s within a single [`TreeMenu`].
pub(crate) fn children_focusables(menu: Entity, queries: &NavQueries) -> NonEmpty<Entity> {
    let ret = children_focusables_helper(menu, queries);
    NonEmpty::try_from(ret)
        .expect("A NavMenu MUST AT LEAST HAVE ONE Focusable child, found one without")
}

fn children_focusables_helper(menu: Entity, queries: &NavQueries) -> Vec<Entity> {
    match queries.children.get(menu).ok() {
        Some(direct_children) => {
            let focusables = direct_children
                .iter()
                .filter(|e| queries.focusables.get(**e).is_ok())
                .cloned();
            let transitive_focusables = direct_children
                .iter()
                .filter(|e| queries.focusables.get(**e).is_err())
                .filter(|e| queries.is_menu.get(**e).is_err())
                .flat_map(|e| children_focusables_helper(*e, queries));
            focusables.chain(transitive_focusables).collect()
        }
        None => Vec::new(),
    }
}

/// Remove all mutually identical elements at the end of `v1` and `v2`.
fn trim_common_tail<T: PartialEq>(v1: &mut NonEmpty<T>, v2: &mut NonEmpty<T>) {
    let mut i1 = v1.len().get() - 1;
    let mut i2 = v2.len().get() - 1;
    loop {
        if v1[i1] != v2[i2] {
            // unwraps: any usize + 1 (saturating) is NonZero
            let l1 = NonZeroUsize::new(i1.saturating_add(1)).unwrap();
            let l2 = NonZeroUsize::new(i2.saturating_add(1)).unwrap();
            v1.truncate(l1);
            v2.truncate(l2);
            return;
        } else if i1 != 0 && i2 != 0 {
            i1 -= 1;
            i2 -= 1;
        } else {
            // There is no changes to be made to the input vectors
            return;
        }
    }
}

fn root_path(mut from: Entity, queries: &NavQueries) -> NonEmpty<Entity> {
    let mut ret = NonEmpty::new(from);
    loop {
        from = match parent_menu(from, queries) {
            // purely personal preference over deeply nested pattern match
            Some((_, menu)) if menu.focus_parent.is_some() => menu.focus_parent.unwrap(),
            _ => return ret,
        };
        assert!(
            !ret.contains(&from),
            "Navigation graph cycle detected! This panic has prevented a stack \
            overflow, please check usages of `NavMenu::reachable_from`"
        );
        ret.push(from);
    }
}

/// Navigate downward the menu hierarchy, traversing all dormant children.
fn focus_deep<'a>(mut menu: &'a TreeMenu, queries: &'a NavQueries) -> Vec<Entity> {
    let mut ret = Vec::with_capacity(4);
    loop {
        let last = menu.active_child;
        ret.insert(0, last);
        menu = match child_menu(last, queries) {
            Some((_, menu)) => menu,
            None => return ret,
        };
    }
}

/// Cycle through a [scoped menu](MenuSetting::BoundScope) according to menu settings
///
/// Returns the index of the element to focus according to `direction`. Cycles
/// if `cycles` and goes over `max_value` or goes bellow 0. `None` if the
/// direction is a dead end.
fn resolve_index(
    from: usize,
    cycles: bool,
    direction: events::ScopeDirection,
    max_value: usize,
) -> Option<usize> {
    use events::ScopeDirection::*;
    match (direction, from) {
        (Previous, 0) => cycles.then(|| max_value),
        (Previous, from) => Some(from - 1),
        (Next, from) if from == max_value => cycles.then(|| 0),
        (Next, from) => Some(from + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::trim_common_tail;
    #[test]
    fn test_trim_common_tail() {
        use non_empty_vec::ne_vec;
        let mut v1 = ne_vec![1, 2, 3, 4, 5, 6, 7];
        let mut v2 = ne_vec![3, 2, 1, 4, 5, 6, 7];
        trim_common_tail(&mut v1, &mut v2);
        assert_eq!(v1, ne_vec![1, 2, 3]);
        assert_eq!(v2, ne_vec![3, 2, 1]);
    }
}
