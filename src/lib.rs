//! A navigation system. Use [`NavigationPlugin`] to get it working
//!
//! See [the RFC](https://github.com/nicopap/rfcs/blob/ui-navigation/rfcs/41-ui-navigation.md)
//! for a deep explanation on how this works.
// TODO: review all uses of `.unwrap()`!
// Notes on the structure of this file:
//
// All "helper functions" are defined after `listen_nav_requests`,
// algorithms are specified over `listen_nav_requests`. While structs and enums
// are defined before all.
pub mod components;
mod events;

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
    // TODO: (for caching and not having to discover all contained focusables
    // just to find the one we are interested in)
    // The child of interest
    //
    // This is a sort of cache to not have to walk down the ECS hierarchy
    // every time we need to find the relevant child.
    // non_inert_child: CacheOption<Entity>,
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
        }
    }

    /// Set this menu as having no parents
    pub fn root() -> Self {
        NavMenu {
            focus_parent: None,
            setting: MenuSetting::ClosedXY,
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
        }
    }
}

/// An [`Entity`] that can be navigated to using the ui navigation system.
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

    /// Programmatically create a `Focusable` with the given state.
    const fn with_state(focus_state: FocusState) -> Self {
        Focusable { focus_state }
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

    let pos_of = |entity: Entity| transform.get(entity).unwrap().translation;
    let focused_pos = transform.get(focused).unwrap().translation.xy();
    let closest = siblings.iter().filter(|sibling| {
        direction.is_in(focused_pos, pos_of(**sibling).xy()) && **sibling != focused
    });
    let closest = max_by_in_iter(closest, |s| -focused_pos.distance_squared(pos_of(**s).xy()));
    match closest {
        None if cycles => {
            let direction = direction.opposite();

            let furthest = siblings.iter().filter(|sibling| {
                direction.is_in(focused_pos, pos_of(**sibling).xy()) && **sibling != focused
            });
            max_by_in_iter(furthest, |s| {
                let pos = pos_of(**s);

                // In a grid, ideally if we are at the leftmost tile and press
                // left, we cycle back ON THE SAME ROW to rightmost tile. To do
                // this, we minimize first `axial_diff` then we care about
                // `focused_pos`.
                //
                // FIXME: this is unoptimal because a very tinny pixel missalignment
                // will cause the cycle to favor a different entity than
                // probably expected.
                // Solution is to look at closest focusable in same direction,
                // but with the X/Y coordinate (corresponding to the direction)
                // set to very low or very high.
                let axial_diff = if matches!(direction, South | North) {
                    (pos.x - focused_pos.x).abs()
                } else {
                    (pos.y - focused_pos.y).abs()
                };
                (-axial_diff, focused_pos.distance_squared(pos.xy()))
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
            let child_menu = queries
                .menus
                .iter()
                .find(|e| e.1.focus_parent == Some(focused));
            let (child_menu, _) = or_none!(child_menu);
            let to = children_focusables(child_menu, queries);
            let to = non_inert_within(&to, queries);
            let to = (*to, from.clone().into()).into();
            NavEvent::FocusChanged { to, from }
        }
        ScopeMove(scope_dir) => {
            let (parent, menu) = or_none!(parent_menu(focused, queries));
            let siblings = children_focusables(parent, queries);
            if !menu.setting.is_scope() {
                let focused = menu.focus_parent.unwrap();
                resolve(focused, request, queries, from.into())
            } else {
                let cycles = menu.setting.cycles();
                let to = or_none!(resolve_scope(focused, scope_dir, cycles, &siblings));
                NavEvent::focus_changed(*to, from)
            }
        }
        FocusOn(new_to_focus) => {
            let mut from = root_path(focused, queries);
            let mut to = root_path(new_to_focus, queries);
            trim_common_tail(&mut from, &mut to);
            NavEvent::FocusChanged { from, to }
        }
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
    // TODO: this most likely breaks when there is more than a single event
    // When no `Focused` found, should take a direct child of a
    // `NavMenu.focus_parent == None`
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
        "A NavMenu MUST AT LEAST HAVE ONE Focusable child, {:?} has none",
        menu
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

/// Remove all but one mutually identical elements at the end of `v1` and `v2`
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
/// assert_eq!(v1, ne_vec![1,2,3,4]);
/// assert_eq!(v2, ne_vec![3,2,1,4]);
/// ```
fn trim_common_tail<T: PartialEq>(v1: &mut NonEmpty<T>, v2: &mut NonEmpty<T>) {
    let mut i1 = v1.len().get() - 1;
    let mut i2 = v2.len().get() - 1;
    loop {
        if v1[i1] != v2[i2] {
            let l1 = NonZeroUsize::new(i1.saturating_add(2)).unwrap();
            let l2 = NonZeroUsize::new(i2.saturating_add(2)).unwrap();
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
        assert_eq!(v1, ne_vec![1, 2, 3, 4]);
        assert_eq!(v2, ne_vec![3, 2, 1, 4]);
    }
}
