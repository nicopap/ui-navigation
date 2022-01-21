//! A navigation system. Use [`NavigationPlugin`] to get it working
//!
//! See [the RFC](https://github.com/nicopap/rfcs/blob/ui-navigation/rfcs/41-ui-navigation.md)
//! for a deep explanation on how this works.
// TODO: review all uses of `.unwrap()`!
// Notes on the structure of this file:
//
// All "helper functions" are defined after `resolve`,
// The algorithm is the `resolve` function and all other functions that
// preceeds it in this file.
mod commands;
pub mod components;
mod events;
pub mod systems;

use std::cmp::Ordering;
use std::fmt;
use std::num::NonZeroUsize;

use bevy::ecs::system::{QuerySingleError, SystemParam};
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
pub use non_empty_vec::NonEmpty;

pub use crate::events::{Direction, NavEvent, NavRequest, ScopeDirection};

#[derive(SystemParam)]
struct NavQueries<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static Parent>,
    focusables: Query<'w, 's, (Entity, &'static Focusable), With<Focusable>>,
    menus: Query<'w, 's, (Entity, &'static NavMenu), With<NavMenu>>,
    transform: Query<'w, 's, &'static GlobalTransform>,
}

/// State of a [`Focusable`]
#[derive(Clone, Debug, Copy, PartialEq)]
pub enum FocusState {
    /// An entity that was previously [`Active`](FocusState::Active) from a branch of
    /// the menu tree that is currently not _focused_. When focus comes back to
    /// the [`NavMenu`] containing this [`Focusable`], the `Dormant` element
    /// will be the [`Focused`](FocusState::Focused) entity.
    Dormant,
    /// The currently highlighted/used entity, there is only a signle _focused_
    /// entity.
    Focused,
    /// The path through the menu tree to the current [`Focused`](FocusState::Focused)
    /// entity. They are the [`Focusable`]s from previous menus that were
    /// activated in order to reach the [`NavMenu`] containing the currently
    /// _focused_ element.
    Active,
    /// None of the above: This [`Focusable`] is neither `Dormant`, `Focused`
    /// or `Active`.
    Inert,
}

#[derive(Clone, Debug, Copy, PartialEq)]
enum MenuSetting {
    /// 2d movement that doesn't cycle
    ClosedXY,
    /// 2d movement cycle on both axis
    CycleXY,
    /// Next/Previous menu without cycle
    ClosedScope,
    /// Next/Previous menu with cycle
    CycleScope,
}
impl MenuSetting {
    fn closed(self) -> Self {
        use MenuSetting::*;
        match self {
            ClosedXY | CycleXY => ClosedXY,
            ClosedScope | CycleScope => ClosedScope,
        }
    }
    fn cycling(self) -> Self {
        use MenuSetting::*;
        match self {
            ClosedXY | CycleXY => CycleXY,
            ClosedScope | CycleScope => CycleScope,
        }
    }
    fn scope(self) -> Self {
        use MenuSetting::*;
        match self {
            ClosedScope | ClosedXY => ClosedScope,
            CycleScope | CycleXY => CycleScope,
        }
    }
    fn cycles(&self) -> bool {
        use MenuSetting::*;
        match self {
            CycleScope | CycleXY => true,
            ClosedScope | ClosedXY => false,
        }
    }
    fn is_2d(&self) -> bool {
        !self.is_scope()
    }
    fn is_scope(&self) -> bool {
        use MenuSetting::*;
        match self {
            ClosedScope | CycleScope => true,
            ClosedXY | CycleXY => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum CacheOption<T> {
    NotYetCached,
    Cached(T),
}
impl<T> From<Option<T>> for CacheOption<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => CacheOption::Cached(value),
            None => CacheOption::NotYetCached,
        }
    }
}
impl<T> From<CacheOption<T>> for Option<T> {
    fn from(value: CacheOption<T>) -> Self {
        match value {
            CacheOption::NotYetCached => None,
            CacheOption::Cached(value) => Some(value),
        }
    }
}

/// A menu that isolate children [`Focusable`]s from other focusables and
/// specify navigation method within itself.
///
/// A `NavMenu` can be used to:
/// * Prevent navigation from one specific submenu to another
/// * Specify the cycle directions of navigation (going left when focusing on a
///   leftmost [`Focusable`] may go to the rightmost `Focusable`)
/// * Specify "scope menus"  such that a [`NavRequest::ScopeMove`] emitted when
///   the focused element is a [`Focusable`] nested within this `NavMenu`
///   will navigate this menu.
/// * Specify _submenus_ and specify from where those submenus are reachable
///
/// # Important
///
/// There are two important invariants to keep in mind:
///
/// 1. There should be **no cycles in the navigation graph**, ie:
///    You must ensure this doesn't create a cycle. You shouldn't be able
///    to reach `NavMenu` X from [`Focusable`] Y if there is a path from
///    `NavMenu` X to `Focusable` Y.
/// 2. There must be **at least one child [`Focusable`]** in the ui graph for each
///    `NavMenu` when sending a [`NavRequest`]
#[derive(Debug, Component, Clone)]
pub struct NavMenu {
    /// The [`Focusable`] in the `NavMenu` that links to this
    /// `NavMenu` (`None` if this `NavMenu` is the menu graph root)
    focus_parent: Option<Entity>,

    /// How we want the user to move between [`Focusable`]s within this menu
    setting: MenuSetting,

    /// This is a sort of cache to not have to walk down the ECS hierarchy
    /// every time we need to find the relevant child.
    non_inert_child: CacheOption<Entity>,
}
impl NavMenu {
    /// Prefer [`NavMenu::reachable_from`] and [`NavMenu::root`] to this
    ///
    /// `new` is useful to programmatically set the parent if you have an
    /// optional value. This saves you from a `match focus_parent`.
    pub fn new(focus_parent: Option<Entity>) -> Self {
        NavMenu {
            focus_parent,
            setting: MenuSetting::ClosedXY,
            non_inert_child: CacheOption::NotYetCached,
        }
    }

    /// Set this menu as having no parents
    pub fn root() -> Self {
        NavMenu {
            focus_parent: None,
            setting: MenuSetting::ClosedXY,
            non_inert_child: CacheOption::NotYetCached,
        }
    }

    /// Set this menu as closed (no cycling)
    pub fn closed(mut self) -> Self {
        self.setting = self.setting.closed();
        self
    }

    /// Set this menu as cycling
    ///
    /// ie: going left from the leftmost element goes to the rightmost element
    pub fn cycling(mut self) -> Self {
        self.setting = self.setting.cycling();
        self
    }

    /// Set this menu as a scope menu
    ///
    /// Meaning: controlled with [`NavRequest::ScopeMove`] even when the
    /// focused element is not in this menu, but in a submenu reachable from
    /// this one.
    pub fn scope(mut self) -> Self {
        self.setting = self.setting.scope();
        self
    }

    /// Set this menu as reachable from a given [`Focusable`]
    ///
    /// When requesting [`NavRequest::Action`] when `focusable` is focused, the
    /// focus will be changed to a focusable within this menu.
    ///
    /// # Important
    ///
    /// You must ensure this doesn't create a cycle. Eg: you shouldn't be able
    /// to reach `NavMenu` X from `Focusable` Y if there is a path from
    /// `NavMenu` X to `Focusable` Y.
    pub fn reachable_from(focusable: Entity) -> Self {
        NavMenu {
            focus_parent: Some(focusable),
            setting: MenuSetting::ClosedXY,
            non_inert_child: CacheOption::NotYetCached,
        }
    }

    fn with_non_inert_child(self, child: Option<Entity>) -> Self {
        NavMenu {
            non_inert_child: child.into(),
            ..self
        }
    }
    fn non_inert_child(&self) -> Option<Entity> {
        self.non_inert_child.into()
    }
}

/// An [`Entity`] that can be navigated to using the ui navigation system.
///
/// It is in one of multiple [`FocusState`], you can check its state with
/// the [`Focusable::state`] method or any of the `is_*` `Focusable` methods.
///
/// A `Focusable` can also be *cancel*. Meaning: when you send the
/// [`NavRequest::Action`] request while a *cancel* `Focusable` is focused,
/// it will act as if the [`NavRequest::Cancel`] request was received.
///
/// To declare a `Focusable` as *cancel*, use the [`Focusable::cancel`]
/// constructor.
#[derive(Component, Clone, Copy)]
pub struct Focusable {
    focus_state: FocusState,
    cancel: bool,
}
impl Default for Focusable {
    fn default() -> Self {
        Focusable {
            focus_state: FocusState::Inert,
            cancel: false,
        }
    }
}
impl fmt::Debug for Focusable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "F({:?})", self.focus_state)
    }
}
impl Focusable {
    /// The state of this `Focusable`
    pub fn state(&self) -> FocusState {
        self.focus_state
    }

    /// This `Focusable` is the unique _focused_ element
    ///
    /// All navigation requests start from it.
    ///
    /// To set an arbitrary [`Focusable`] to _focused_, you should send a
    /// [`NavRequest::FocusOn`] request.
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
    /// encompassing [`NavMenu`], the _focused_ element will be the _dormant_
    /// element within the menu.
    pub fn is_dormant(&self) -> bool {
        self.focus_state == FocusState::Dormant
    }

    /// This `Focusable` is neither _active_, _focused_ or _dormant_
    pub fn is_inert(&self) -> bool {
        self.focus_state == FocusState::Inert
    }

    /// This `Focusable` is a cancel button, see [`Focusable::cancel`]
    pub fn is_cancel(&self) -> bool {
        self.cancel
    }

    /// This is a "Cancel" button, whenever a [`NavRequest::Action`] is sent
    /// while this [`Focusable`] is _focused_, act as if the request was a
    /// [`NavRequest::Cancel`]
    pub fn cancel() -> Self {
        Focusable {
            focus_state: FocusState::Inert,
            cancel: true,
        }
    }
}

/// The currently _focused_ [`Focusable`]
///
/// You cannot edit it or create new `Focused` component. To set an arbitrary
/// [`Focusable`] to _focused_, you should send a [`NavRequest::FocusOn`]
/// request.
///
/// This [`Component`] is useful if you need to query for the _currently
/// focused_ element using a `Query<Entity, With<Focused>>` [`SystemParam`] for
/// example.
///
/// You can also check if a [`Focusable`] is _focused_ using
/// [`Focusable::is_focused`].
#[derive(Component)]
#[non_exhaustive]
pub struct Focused;

/// Which `Entity` in `siblings` can be reached from `focused` in
/// `direction` given entities `transform` if any, otherwise `None`
fn resolve_2d<'a, 'b, 'c>(
    focused: Entity,
    direction: Direction,
    cycles: bool,
    siblings: &'a [Entity],
    transform: &'b Query<&'c GlobalTransform>,
) -> Option<&'a Entity> {
    use Direction::*;

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

fn max_by_in_iter<U, T: PartialOrd>(
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
    direction: ScopeDirection,
    cycles: bool,
    siblings: &NonEmpty<Entity>,
) -> Option<&Entity> {
    let focused_index = siblings
        .iter()
        .enumerate()
        .find(|e| *e.1 == focused)
        .map(|e| e.0)?;
    let new_index = resolve_index(focused_index, cycles, direction, siblings.len().get() - 1);
    new_index.and_then(|i| siblings.get(i))
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
                Some(val) => (Some(val.0), val.1.setting.cycles()),
                None => (None, true),
            };
            let siblings = match parent {
                Some(parent) => children_focusables(parent, queries),
                None if !queries.focusables.is_empty() => {
                    let focusables: Vec<_> = queries.focusables.iter().map(|tpl| tpl.0).collect();
                    NonEmpty::try_from(focusables).unwrap()
                }
                None => {
                    panic!("There must be at least one `Focusable` when sending a `NavRequest`!")
                }
            };
            let to = resolve_2d(focused, direction, cycles, &siblings, &queries.transform);
            let to = or_none!(to);
            NavEvent::focus_changed(*to, from)
        }
        Cancel => {
            let to = or_none!(parent_menu(focused, queries));
            let to = or_none!(to.1.focus_parent);
            from.push(to);
            NavEvent::focus_changed(to, from)
        }
        Action => {
            if let Ok((_, focusable)) = queries.focusables.get(focused) {
                if focusable.cancel {
                    let mut from = from.to_vec();
                    from.truncate(from.len() - 1);
                    return resolve(focused, NavRequest::Cancel, queries, from);
                }
            }
            let child_menu = child_menu(focused, queries);
            let (child_menu, menu) = or_none!(child_menu);
            let to = menu.non_inert_child().unwrap_or_else(|| {
                let ret = children_focusables(child_menu, queries);
                *non_inert_within(&ret, queries)
            });
            let to = (to, from.clone().into()).into();
            NavEvent::FocusChanged { to, from }
        }
        ScopeMove(scope_dir) => {
            let (parent, menu) = or_none!(parent_menu(focused, queries));
            let siblings = children_focusables(parent, queries);
            if !menu.setting.is_scope() {
                let focused = or_none!(menu.focus_parent);
                resolve(focused, request, queries, from.into())
            } else {
                let cycles = menu.setting.cycles();
                let to = or_none!(resolve_scope(focused, scope_dir, cycles, &siblings));
                let extra = match child_menu(*to, queries) {
                    Some((_, menu)) => focus_deep(menu.clone(), queries),
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
    }
}

/// Set the [`non_inert_child`](NavMenu::non_inert_child) field of the enclosing [`NavMenu`]
/// and disables the previous one
fn cache_non_inert(child: Entity, queries: &NavQueries, cmds: &mut Commands) {
    if let Some((menu, nav_menu)) = parent_menu(child, queries) {
        if let Some(entity) = nav_menu.non_inert_child() {
            cmds.add(commands::set_focus_state(entity, FocusState::Inert));
        }
        let updated_menu = nav_menu.with_non_inert_child(Some(child));
        cmds.entity(menu).insert(updated_menu);
    }
}

/// Listen to [`NavRequest`] and update the state of [`Focusable`] entities if
/// relevant.
fn listen_nav_requests(
    focused: Query<Entity, With<Focused>>,
    mut requests: EventReader<NavRequest>,
    queries: NavQueries,
    mut events: EventWriter<NavEvent>,
    mut commands: Commands,
) {
    use FocusState as Fs;
    // TODO: this most likely breaks when there is more than a single event,
    // since we use the `commands` interface to mutate the `Focused` element
    // and change component values.
    for request in requests.iter() {
        let focused_id = focused.get_single().unwrap_or_else(|err| {
            assert!(
                !matches!(err, QuerySingleError::MultipleEntities(_)),
                "Multiple entities with Focused component, this should not happen"
            );
            queries.focusables.iter().next().unwrap().0
        });
        let event = resolve(focused_id, *request, &queries, Vec::new());
        // Change focus state of relevant entities
        if let NavEvent::FocusChanged { to, from } = &event {
            if to == from {
                continue;
            }
            let (&disable, put_to_sleep) = from.split_last();
            commands.add(commands::set_focus_state(disable, Fs::Inert));
            for &entity in put_to_sleep {
                cache_non_inert(entity, &queries, &mut commands);
                commands.add(commands::set_focus_state(entity, Fs::Dormant));
            }

            let (&focus, activate) = to.split_first();
            cache_non_inert(focus, &queries, &mut commands);
            commands.add(commands::set_focus_state(focus, Fs::Focused));
            for &entity in activate {
                cache_non_inert(entity, &queries, &mut commands);
                commands.add(commands::set_focus_state(entity, Fs::Active));
            }
        };
        events.send(event);
    }
}

/// The child [`NavMenu`] of `focusable`
fn child_menu<'a>(focusable: Entity, queries: &'a NavQueries) -> Option<(Entity, &'a NavMenu)> {
    queries
        .menus
        .iter()
        .find(|e| e.1.focus_parent == Some(focusable))
}

/// The [`NavMenu`] containing `focusable`, if any
fn parent_menu(focusable: Entity, queries: &NavQueries) -> Option<(Entity, NavMenu)> {
    let Parent(parent) = queries.parents.get(focusable).ok()?;
    match queries.menus.get(*parent) {
        Ok(menu) => Some((*parent, menu.1.clone())),
        Err(_) => parent_menu(*parent, queries),
    }
}

/// All sibling [`Focusable`]s within a single [`NavMenu`]
fn children_focusables(menu: Entity, queries: &NavQueries) -> NonEmpty<Entity> {
    let ret = children_focusables_helper(menu, queries);
    assert!(
        !ret.is_empty(),
        "A NavMenu MUST AT LEAST HAVE ONE Focusable child, {menu:?} has none",
    );
    NonEmpty::try_from(ret).unwrap()
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
                .filter(|e| queries.menus.get(**e).is_err())
                .flat_map(|e| children_focusables_helper(*e, queries));
            focusables.chain(transitive_focusables).collect()
        }
        None => Vec::new(),
    }
}

/// Which `Entity` in `siblings` is not _inert_, or the first in `siblings` if
/// none found.
fn non_inert_within<'a, 'b>(siblings: &'a NonEmpty<Entity>, queries: &'b NavQueries) -> &'a Entity {
    siblings
        .iter()
        .find(|e| queries.focusables.get(**e).iter().any(|f| !f.1.is_inert()))
        .unwrap_or_else(|| siblings.first())
}

/// Remove all mutually identical elements at the end of `v1` and `v2`
///
/// # Example
///
/// ```rust,ignore
/// # use non_empty_vec::ne_vec;
/// let mut v1 = ne_vec![1,2,3,4,5,6,7];
/// let mut v2 = ne_vec![3,2,1,4,5,6,7];
///
/// trim_common_tail(&mut v1, &mut v2);
///
/// assert_eq!(v1, ne_vec![1,2,3]);
/// assert_eq!(v2, ne_vec![3,2,1]);
/// ```
fn trim_common_tail<T: PartialEq>(v1: &mut NonEmpty<T>, v2: &mut NonEmpty<T>) {
    let mut i1 = v1.len().get() - 1;
    let mut i2 = v2.len().get() - 1;
    loop {
        if v1[i1] != v2[i2] {
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

/// Navigate downward the menu hierarchy N steps, and return the path to the
/// last level reached
fn focus_deep(mut menu: NavMenu, queries: &NavQueries) -> Vec<Entity> {
    let mut ret = Vec::with_capacity(4);
    loop {
        let last = match menu.non_inert_child() {
            Some(additional) => {
                ret.insert(0, additional);
                additional
            }
            None => return ret,
        };
        menu = match child_menu(last, queries) {
            Some((_, menu)) => menu.clone(),
            None => return ret,
        };
    }
}

/// Cycle through a [scoped menu](MenuSetting::CycleScope) according to menu settings
///
/// Returns the index of the element to focus according to `direction`. Cycles
/// if `cycles` and goes over `max_value` or goes bellow 0. `None` if the
/// direction is a dead end.
fn resolve_index(
    from: usize,
    cycles: bool,
    direction: ScopeDirection,
    max_value: usize,
) -> Option<usize> {
    use ScopeDirection::*;
    match (direction, cycles, from) {
        (Previous, true, 0) => Some(max_value),
        (Previous, true, from) => Some(from - 1),
        (Previous, false, 0) => None,
        (Previous, false, from) => Some(from - 1),
        (Next, true, from) => Some((from + 1) % (max_value + 1)),
        (Next, false, from) if from == max_value => None,
        (Next, false, from) => Some(from + 1),
    }
}

/// The navigation plugin
///
/// Add it to your app with `.add_plugin(NavigationPlugin)` and send
/// [`NavRequest`]s to move focus within declared [`Focusable`] entities.
pub struct NavigationPlugin;
impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NavRequest>()
            .add_event::<NavEvent>()
            // TODO: add label to system so that it can be sorted
            .add_system(listen_nav_requests);
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
