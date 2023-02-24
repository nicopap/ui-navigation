//! The resolution algorithm for the navigation system.
//!
//! # Overview
//!
//! This module defines two systems:
//! 1. [`listen_nav_requests`]: the system gathering [`NavRequest`] and running
//!    the [`resolve`] algorithm on them, updating the [`Focusable`] states and
//!    sending [`NavEvent`] as result.
//! 2. [`insert_tree_menus`]: system responsible to convert [`MenuBuilder`] defined
//!    in [`crate::menu`] into [`TreeMenu`], which is the component used by
//!    the resolution algorithm.
//!
//! The module also defines the [`Focusable`] component (also used in the
//! resolution algorithm) and its fields.
//!
//! The bulk of the resolution algorithm is implemented in [`resolve`],
//! delegating some abstract tasks to helper functions, of which:
//! * [`parent_menu`]
//! * [`ChildQueries::focusables_of`]
//! * [`child_menu`]
//! * [`focus_deep`]
//! * [`MenuNavigationStrategy::resolve_2d`]
//! * [`resolve_scope`]
//!
//! A trait [`MenuNavigationStrategy`] allows user-defined movements
//! through a custom system parameter by implementing `resolve_2d`.
//!
//! We define some `SystemParam`:
//! * [`ChildQueries`]: queries used to find the focusable children of a given entity.
//! * [`NavQueries`]: All **immutable** queries used by the resolution algorithm.
//! * [`MutQueries`]: Queries with mutable access to [`Focusable`] and [`TreeMenu`]
//!   for updating them in [`listen_nav_requests`].
//! * [`UiProjectionQuery`]: A default implementation of [`MenuNavigationStrategy`]
//!   for `bevy_ui`.
//!
//! [`listen_nav_requests`] uses a `ParamSet` to access the focusables immutably for
//! navigation resolution and mutably for updating them with the new navigation state.
use std::num::NonZeroUsize;

use bevy::hierarchy::{Children, Parent};
use bevy::log::warn;
use bevy::prelude::Changed;
#[cfg(feature = "bevy_reflect")]
use bevy::reflect::{FromReflect, Reflect};
use bevy::{
    ecs::{
        event::{EventReader, EventWriter},
        prelude::{Commands, Component, Entity, ParamSet, Query, ResMut, With, Without},
        system::{Resource, StaticSystemParam, SystemParam, SystemParamItem},
    },
    math::Vec2,
};
#[cfg(feature = "bevy_ui")]
use bevy::{
    math::Vec3Swizzles,
    prelude::{GlobalTransform, Res},
    utils::FloatOrd,
};

use non_empty_vec::NonEmpty;

use crate::{
    commands::set_focus_state,
    events::{self, NavEvent, NavRequest},
    menu::{MenuBuilder, MenuSetting},
};

/// System parameter used to resolve movement and cycling focus updates.
///
/// This is useful if you don't want to depend
/// on bevy's `GlobalTransform` for your UI,
/// or want to implement your own navigation algorithm.
/// For example, if you want your ui to be 3d elements in the world.
pub trait MenuNavigationStrategy {
    /// Which [`Entity`] in `siblings` can be reached
    /// from `focused` in `direction` if any, otherwise `None`.
    ///
    /// * `focused`: The currently focused entity in the menu
    /// * `direction`: The direction in which the focus should move
    /// * `cycles`: Whether the navigation should loop
    /// * `sibligns`: All the other focusable entities in this menu
    ///
    /// Note that `focused` appears once in `siblings`.
    fn resolve_2d<'a>(
        &self,
        focused: Entity,
        direction: events::Direction,
        cycles: bool,
        siblings: &'a [Entity],
    ) -> Option<&'a Entity>;
}

/// A rectangle to specify the [`ScreenBoundaries`],
/// useful for 2d navigation wrapping.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct Rect {
    /// The higher `x,y` coordinate of the `Rect`.
    pub max: Vec2,
    /// The lower `x,y` coordinate of the `Rect`.
    pub min: Vec2,
}
/// Specify the boundaries of the screen when using 2d wrapping navigation.
///
/// This will be used in the default [`MenuNavigationStrategy`].
///
/// **NOTE**: This is deprecated since `bevy_ui` doesn't support moving
/// the UI camera anymore.
#[derive(Debug, Clone, Copy, Resource)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct ScreenBoundaries {
    /// Position of the camera.
    pub position: Vec2,
    /// The borders of the camera.
    pub screen_edge: Rect,
    /// The zoom level of the camera.
    pub scale: f32,
}

#[derive(SystemParam)]
pub(crate) struct ChildQueries<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    is_focusable: Query<'w, 's, &'static Focusable>,
    is_menu: Query<'w, 's, With<MenuSetting>>,
}

/// System parameter for the default cursor navigation system.
///
/// It uses the bevy [`GlobalTransform`] to compute relative positions
/// and change focus to the correct entity.
/// It uses the [`ScreenBoundaries`] resource to compute screen boundaries
/// and move the cursor accordingly when it reaches a screen border
/// in a cycling menu.
#[cfg(feature = "bevy_ui")]
#[derive(SystemParam)]
pub struct UiProjectionQuery<'w, 's> {
    boundaries: Option<Res<'w, ScreenBoundaries>>,
    transforms: Query<'w, 's, &'static GlobalTransform>,
}

/// Collection of queries to manage the navigation tree.
#[allow(clippy::type_complexity)]
#[derive(SystemParam)]
pub(crate) struct NavQueries<'w, 's> {
    pub(crate) children: ChildQueries<'w, 's>,
    parents: Query<'w, 's, &'static Parent>,
    focusables: Query<'w, 's, (Entity, &'static Focusable), Without<TreeMenu>>,
    menus: Query<'w, 's, (Entity, &'static TreeMenu, &'static MenuSetting), Without<Focusable>>,
}
impl<'w, 's> NavQueries<'w, 's> {
    fn active_menu(
        &self,
        mut entity: Entity,
        mut active_child: Entity,
    ) -> Option<(Entity, Entity)> {
        let mut repeated = false;
        loop {
            let mut go_down_one_menu = || {
                let (_, focus) = self.focusables.get(active_child).ok()?;
                if focus.state() != FocusState::Active {
                    return None;
                }
                let (new_menu_entity, child_menu, _) = child_menu(active_child, self)?;
                repeated = true;
                entity = new_menu_entity;
                active_child = child_menu.active_child;
                Some(())
            };
            match go_down_one_menu() {
                Some(()) => {}
                None if !repeated => return None,
                None => return Some((entity, active_child)),
            }
        }
    }

    /// The [`TreeMenu`] containing `focusable`, if any.
    pub(crate) fn parent_menu(&self, focusable: Entity) -> Option<(Entity, TreeMenu, MenuSetting)> {
        let parent = self.parents.get(focusable).ok()?.get();
        match self.menus.get(parent) {
            Ok((_, tree, setting)) => Some((parent, tree.clone(), *setting)),
            Err(_) => self.parent_menu(parent),
        }
    }

    // TODO: worst case this iterates 3 times through list of focusables and once menus.
    // Could be improved to a single pass.
    fn pick_first_focused(&self) -> Option<Entity> {
        use FocusState::{Blocked, Focused, Inert};
        let iter_focused = || self.focusables.iter().filter(|f| f.1.state() != Blocked);
        let root_menu = || {
            self.menus
                .iter()
                .find(|(_, menu, _)| menu.focus_parent.is_none())
        };
        let any_in_menu = |entity, active_child| {
            match self.focusables.get(active_child) {
                Ok((entity, _)) => Some(entity),
                // TODO: non-Inert non-active_child
                Err(_) => self.children.focusables_of(entity).first().copied(),
            }
        };
        let any_in_active = || {
            let (root_menu_entity, menu, _) = root_menu()?;
            let (active_menu_entity, active) =
                self.active_menu(root_menu_entity, menu.active_child)?;
            any_in_menu(active_menu_entity, active)
        };
        let any_in_root = || {
            let (root_menu_entity, menu, _) = root_menu()?;
            any_in_menu(root_menu_entity, menu.active_child)
        };
        let any_prioritized =
            || iter_focused().find_map(|(e, focus)| (focus.state != Inert).then(|| e));
        let fallback = || iter_focused().next().map(|(fo, _)| fo);
        let focused = iter_focused().find_map(|(fo, focus)| (focus.state == Focused).then(|| fo));

        focused
            .or_else(any_in_active)
            .or_else(any_prioritized)
            .or_else(any_in_root)
            .or_else(fallback)
    }

    fn root_path(&self, mut from: Entity) -> NonEmpty<Entity> {
        let mut ret = NonEmpty::new(from);
        loop {
            from = match self.parent_menu(from) {
                // purely personal preference over deeply nested pattern match
                Some((_, menu, _)) if menu.focus_parent.is_some() => menu.focus_parent.unwrap(),
                _ => return ret,
            };
            assert!(
                !ret.contains(&from),
                "Navigation graph cycle detected! This panic has prevented a stack \
                overflow, please check usages of `MenuBuilder::Entity/NamedParent`"
            );
            ret.push(from);
        }
    }
}

/// Queries [`Focusable`] and [`TreeMenu`] in a mutable way.
#[derive(SystemParam)]
pub(crate) struct MutQueries<'w, 's> {
    commands: Commands<'w, 's>,
    parents: Query<'w, 's, &'static Parent>,
    focusables: Query<'w, 's, &'static mut Focusable, Without<TreeMenu>>,
    menus: Query<'w, 's, &'static mut TreeMenu, Without<Focusable>>,
}
impl<'w, 's> MutQueries<'w, 's> {
    /// Set the [`active_child`](TreeMenu::active_child) field of the enclosing
    /// [`TreeMenu`] and disables the previous one.
    fn set_active_child(&mut self, child: Entity) {
        let mut focusable = child;
        let mut nav_menu = loop {
            // Find the enclosing parent menu.
            if let Ok(parent) = self.parents.get(focusable) {
                let parent = parent.get();
                focusable = parent;
                if let Ok(menu) = self.menus.get_mut(parent) {
                    break menu;
                }
            } else {
                return;
            }
        };
        let entity = nav_menu.active_child;
        nav_menu.active_child = child;
        self.set_entity_focus(entity, FocusState::Inert);
    }

    fn set_entity_focus(&mut self, entity: Entity, state: FocusState) {
        if let Ok(mut focusable) = self.focusables.get_mut(entity) {
            focusable.state = state;
            self.commands.add(set_focus_state(entity, state));
        }
    }

    /// Change focus state of relevant entities.
    fn update_focus(&mut self, from: &[Entity], to: &NonEmpty<Entity>) -> Entity {
        use FocusState as Fs;

        if to.as_slice() == from {
            return *to.first();
        }
        let (disable, put_to_sleep) = from
            .split_last()
            .map_or((None, from), |(tail, heads)| (Some(tail), heads));
        if let Some(disable) = disable {
            self.set_entity_focus(*disable, Fs::Inert);
        }
        for &entity in put_to_sleep {
            self.set_entity_focus(entity, Fs::Prioritized);
        }
        let (&focus, activate) = to.split_first();
        self.set_active_child(focus);
        self.set_entity_focus(focus, Fs::Focused);
        for &entity in activate {
            self.set_active_child(entity);
            self.set_entity_focus(entity, Fs::Active);
        }
        focus
    }
}

/// State of a [`Focusable`].
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub enum FocusState {
    /// An entity that was previously [`FocusState::Active`]
    /// from a branch of the menu tree that is currently not _focused_.
    /// When focus comes back to the [`MenuSetting`] containing this [`Focusable`],
    /// the `Prioritized` element will be the [`FocusState::Focused`] entity.
    Prioritized,

    /// The currently highlighted/used entity,
    /// there is only a signle _focused_ entity.
    ///
    /// All navigation requests start from it.
    ///
    /// To set an arbitrary [`Focusable`] to _focused_, you should send a
    /// [`NavRequest::FocusOn`] request.
    Focused,

    /// This [`Focusable`] is on the path in the menu tree
    /// to the current [`FocusState::Focused`] entity.
    ///
    /// [`FocusState::Active`] focusables are the [`Focusable`]s
    /// from previous menus that were activated
    /// in order to reach the [`MenuSetting`] containing
    /// the currently _focused_ element.
    ///
    /// It's the "breadcrumb" of buttons to activate to reach
    /// the currently focused element from the root menu.
    Active,

    /// Prevents all interactions with this [`Focusable`].
    ///
    /// This is equivalent to removing the `Focusable` component
    /// from the entity, but without the latency.
    Blocked,

    /// None of the above:
    /// This [`Focusable`] is neither `Prioritized`, `Focused` or `Active`.
    Inert,
}

/// The reason why the navigation system is locked.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect, FromReflect))]
pub enum LockReason {
    /// Navigation was locked by activating a [lock focusable].
    ///
    /// [lock focusable] Focusable::lock
    Focusable(Entity),

    /// Navigation was locked by sending a [`NavRequest::Lock`].
    NavRequest,
}

/// The navigation system's lock.
///
/// When locked, the navigation system doesn't process any [`NavRequest`].
/// It only waits on a [`NavRequest::Unlock`] event. It will then continue
/// processing new requests.
#[derive(Resource)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct NavLock {
    lock_reason: Option<LockReason>,
}
impl NavLock {
    pub(crate) fn new() -> Self {
        Self { lock_reason: None }
    }
    /// The reason why navigation is locked, `None` if currently unlocked.
    pub fn reason(&self) -> Option<LockReason> {
        self.lock_reason
    }
    /// Whether the navigation system is locked.
    pub fn is_locked(&self) -> bool {
        self.lock_reason.is_some()
    }
}

/// A menu that isolate children [`Focusable`]s from other focusables
/// and specify navigation method within itself.
///
/// The user can't create a `TreeMenu`,
/// they will use the [`MenuSetting`] API
/// and the `TreeMenu` component will be inserted
/// by the [`insert_tree_menus`] system.
#[derive(Debug, Component, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub(crate) struct TreeMenu {
    /// The [`Focusable`] that sends to this `MenuSetting`
    /// when receiving [`NavRequest::Action`].
    pub(crate) focus_parent: Option<Entity>,
    /// The currently prioritized or active focusable in this menu.
    pub(crate) active_child: Entity,
}

/// The actions triggered by a [`Focusable`].
#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
#[non_exhaustive]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub enum FocusAction {
    /// Acts like a standard navigation node.
    ///
    /// Goes into relevant menu if any [`MenuSetting`] is
    /// [_reachable from_](MenuBuilder::from_named) this [`Focusable`].
    #[default]
    Normal,

    /// If we receive [`NavRequest::Action`]
    /// while this [`Focusable`] is focused,
    /// it will act as a [`NavRequest::Cancel`]
    /// (leaving submenu to enter the parent one).
    Cancel,

    /// If we receive [`NavRequest::Action`]
    /// while this [`Focusable`] is focused,
    /// the navigation system will freeze
    /// until [`NavRequest::Unlock`] is received,
    /// sending a [`NavEvent::Unlocked`].
    ///
    /// This is useful to implement widgets with complex controls
    /// you don't want to accidentally unfocus,
    /// or suspending the navigation system while in-game.
    Lock,
}

/// An [`Entity`] that can be navigated to, using the cursor navigation system.
///
/// It is in one of multiple [`FocusState`],
/// you can check its state with the [`Focusable::state`] method.
///
/// A `Focusable` can execute a variety of [`FocusAction`]
/// when receiving [`NavRequest::Action`],
/// the default one is [`FocusAction::Normal`].
///
/// **Note**: You should avoid updating manually the state of [`Focusable`]s.
/// You should instead use [`NavRequest`] to manipulate and change focus.
#[derive(Component, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
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
impl Focusable {
    /// Default Focusable.
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

    /// A "cancel" focusable, see [`FocusAction::Cancel`].
    pub fn cancel() -> Self {
        Focusable {
            state: FocusState::Inert,
            action: FocusAction::Cancel,
        }
    }
    /// A "lock" focusable, see [`FocusAction::Lock`].
    pub fn lock() -> Self {
        Focusable {
            state: FocusState::Inert,
            action: FocusAction::Lock,
        }
    }
    /// A focusable that will get highlighted in priority when none are set yet.
    ///
    /// **WARNING**: Only use this when creating the UI.
    /// Any of the following state is unspecified
    /// and will likely result in broken behavior:
    /// * Having multiple prioritized `Focusable`s in the same menu.
    /// * Updating an already existing `Focusable` with this.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bevy_ui_navigation::prelude::Focusable;
    /// # use bevy_ui_navigation::components::FocusableButtonBundle;
    /// # use bevy::prelude::*;
    /// fn setup(mut commands: Commands) {
    ///     commands.spawn(FocusableButtonBundle {
    ///         focus: Focusable::new().prioritized(),
    ///         ..default()
    ///     });
    /// }
    /// ```
    pub fn prioritized(self) -> Self {
        Self {
            state: FocusState::Prioritized,
            ..self
        }
    }

    /// A [`FocusState::Blocked`] focusable.
    ///
    /// This focusable will not be able to take focus until
    /// [`Focusable::unblock`] is called on it.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bevy_ui_navigation::prelude::Focusable;
    /// # use bevy_ui_navigation::components::FocusableButtonBundle;
    /// # use bevy::prelude::*;
    /// fn setup(mut commands: Commands) {
    ///     commands.spawn(FocusableButtonBundle {
    ///         focus: Focusable::new().blocked(),
    ///         ..default()
    ///     });
    /// }
    /// ```
    pub fn blocked(self) -> Self {
        Self {
            state: FocusState::Blocked,
            ..self
        }
    }

    /// Prevent this [`Focusable`] from gaining focus until it is unblocked.
    ///
    /// **Note**: Due to the way focus is handled, this does nothing
    /// when the [`Focusable::state`] is [`FocusState::Active`]
    /// or [`FocusState::Focused`].
    ///
    /// Returns `true` if `self` has succesfully been blocked
    /// (its [`Focusable::state`] was either `Inert` or `Prioritized`).
    ///
    /// # Limitations
    ///
    /// - If all the children of a menu are blocked, when activating the menu's
    ///   parent, the block state of the last active focusable will be ignored.
    /// - When `FocusOn` to an focusable in a menu reachable from an blocked
    ///   focusable, its block state will be ignored.
    pub fn block(&mut self) -> bool {
        use FocusState::{Blocked, Inert, Prioritized};
        let blockable = matches!(self.state(), Inert | Prioritized);
        if blockable {
            self.state = Blocked;
        }
        blockable
    }

    /// Allow this [`Focusable`] to gain focus again,
    /// setting it to [`FocusState::Inert`].
    ///
    /// Returns `true` if `self`'s state was [`FocusState::Blocked`].
    pub fn unblock(&mut self) -> bool {
        if self.state() == FocusState::Blocked {
            self.state = FocusState::Inert;
            true
        } else {
            false
        }
    }
}

/// The currently _focused_ [`Focusable`].
///
/// You cannot edit it or create new `Focused` component.
/// To set an arbitrary [`Focusable`] to _focused_,
/// you should send [`NavRequest::FocusOn`].
///
/// This [`Component`] is useful
/// if you needto query for the _currently focused_ element,
/// using `Query<Entity, With<Focused>>` for example.
///
/// If a [`Focusable`] is focused,
/// its [`Focusable::state()`] will be [`FocusState::Focused`],
///
/// # Notes
///
/// The `Focused` marker component is only updated
/// at the end of the `CoreStage::Update` stage.
/// This means it might lead to a single frame of latency
/// compared to using [`Focusable::state()`].
#[derive(Component)]
#[non_exhaustive]
pub struct Focused;

#[cfg(feature = "bevy_ui")]
impl<'w, 's> MenuNavigationStrategy for UiProjectionQuery<'w, 's> {
    fn resolve_2d<'a>(
        &self,
        focused: Entity,
        direction: events::Direction,
        cycles: bool,
        siblings: &'a [Entity],
    ) -> Option<&'a Entity> {
        use events::Direction::*;

        let pos_of = |entity: Entity| {
            self.transforms
                .get(entity)
                .expect("Focusable entities must have a GlobalTransform component")
                .translation()
                .xy()
        };
        let focused_pos = pos_of(focused);
        let closest = siblings
            .iter()
            .filter(|sibling| {
                direction.is_in(focused_pos, pos_of(**sibling)) && **sibling != focused
            })
            .max_by_key(|s| FloatOrd(-focused_pos.distance_squared(pos_of(**s))));
        match (closest, self.boundaries.as_ref()) {
            (None, None) if cycles => {
                warn!(
                    "Tried to move in {direction:?} from Focusable {focused:?} while no other \
                 Focusables were there. There were no `Res<ScreenBoundaries>`, so we couldn't \
                 compute the screen edges for cycling. Make sure you either add the \
                 bevy_ui_navigation::systems::update_boundaries system to your app or implement \
                 your own routine to manage a `Res<ScreenBoundaries>`."
                );
                None
            }
            (None, Some(boundaries)) if cycles => {
                let (x, y) = (boundaries.position.x, boundaries.position.y);
                let edge = boundaries.screen_edge;
                let scale = boundaries.scale;
                let focused_pos = match direction {
                    // NOTE: up/down axises are inverted in bevy
                    South => Vec2::new(focused_pos.x, y - scale * edge.min.y),
                    North => Vec2::new(focused_pos.x, y + scale * edge.max.y),
                    East => Vec2::new(x - edge.min.x * scale, focused_pos.y),
                    West => Vec2::new(x + edge.max.x * scale, focused_pos.y),
                };
                siblings
                    .iter()
                    .max_by_key(|s| FloatOrd(-focused_pos.distance_squared(pos_of(**s))))
            }
            (anyelse, _) => anyelse,
        }
    }
}

/// Returns the next or previous entity based on `direction`.
fn resolve_scope(
    focused: Entity,
    direction: events::ScopeDirection,
    cycles: bool,
    siblings: &[Entity],
) -> Option<&Entity> {
    let focused_index = siblings.iter().position(|e| *e == focused)?;
    let new_index = resolve_index(focused_index, cycles, direction, siblings.len() - 1);
    new_index.and_then(|i| siblings.get(i))
}

/// Find the event created by `request` where the focused element is `focused`.
fn resolve<STGY: MenuNavigationStrategy>(
    focused: Entity,
    request: NavRequest,
    queries: &NavQueries,
    // this is to avoid triggering change detection if not updated.
    lock: &mut ResMut<NavLock>,
    from: Vec<Entity>,
    strategy: &STGY,
) -> NavEvent {
    use FocusState::Blocked;
    use NavRequest::*;

    assert!(
        queries.focusables.get(focused).is_ok(),
        "The resolution algorithm MUST go from a focusable element"
    );
    assert!(
        !from.contains(&focused),
        "Navigation graph cycle detected! This panic has prevented a stack overflow, \
        please check usages of `MenuSetting::reachable_from`"
    );

    let mut from = (from, focused).into();

    // Early exit with a `NoChanges` event.
    macro_rules! or_none {
        ($to_match:expr) => {
            match $to_match {
                Some(x) => x,
                None => return NavEvent::NoChanges { from, request },
            }
        };
    }
    match request {
        Lock => {
            if lock.is_locked() {
                return NavEvent::NoChanges { from, request };
            }
            let reason = LockReason::NavRequest;
            lock.lock_reason = Some(reason);
            NavEvent::Locked(reason)
        }
        Move(direction) => {
            let (parent, cycles) = match queries.parent_menu(focused) {
                Some(val) if !val.2.is_2d() => return NavEvent::NoChanges { from, request },
                Some(val) => (Some(val.0), !val.2.bound()),
                None => (None, true),
            };
            let unblocked = |(e, focus): (_, &Focusable)| (focus.state != Blocked).then(|| e);
            let siblings = match parent {
                Some(parent) => queries.children.focusables_of(parent),
                None => queries.focusables.iter().filter_map(unblocked).collect(),
            };
            let to = strategy.resolve_2d(focused, direction, cycles, &siblings);
            NavEvent::focus_changed(*or_none!(to), from)
        }
        Cancel => {
            let to = or_none!(queries.parent_menu(focused));
            let to = or_none!(to.1.focus_parent);
            from.push(to);
            NavEvent::focus_changed(to, from)
        }
        Action => {
            match queries.focusables.get(focused).map(|e| e.1.action) {
                Ok(FocusAction::Cancel) => {
                    let mut from = from.to_vec();
                    from.truncate(from.len() - 1);
                    return resolve(focused, NavRequest::Cancel, queries, lock, from, strategy);
                }
                Ok(FocusAction::Lock) => {
                    let reason = LockReason::Focusable(focused);
                    lock.lock_reason = Some(reason);
                    return NavEvent::Locked(reason);
                }
                Err(_) | Ok(FocusAction::Normal) => {}
            }
            let child_menu = child_menu(focused, queries);
            let (_, menu, _) = or_none!(child_menu);
            let to = (menu.active_child, from.clone().into()).into();
            NavEvent::FocusChanged { to, from }
        }
        // "Tab move" nested movement
        ScopeMove(scope_dir) => {
            let (parent, menu, setting) = or_none!(queries.parent_menu(focused));
            let siblings = queries.children.focusables_of(parent);
            if !setting.is_scope() {
                let focused = or_none!(menu.focus_parent);
                resolve(focused, request, queries, lock, from.into(), strategy)
            } else {
                let cycles = !setting.bound();
                let to = or_none!(resolve_scope(focused, scope_dir, cycles, &siblings));
                let extra = match child_menu(*to, queries) {
                    Some((_, menu, _)) => focus_deep(menu, queries),
                    None => Vec::new(),
                };
                let to = (extra, *to).into();
                NavEvent::FocusChanged { to, from }
            }
        }
        FocusOn(new_to_focus) => {
            // assumption here is that there is a common ancestor
            // though nothing really breaks if there isn't
            let mut from = queries.root_path(focused);
            let mut to = queries.root_path(new_to_focus);
            trim_common_tail(&mut from, &mut to);
            if from == to {
                NavEvent::NoChanges { from, request }
            } else {
                NavEvent::FocusChanged { from, to }
            }
        }
        Unlock => {
            if let Some(lock_entity) = lock.lock_reason.take() {
                NavEvent::Unlocked(lock_entity)
            } else {
                warn!("Received a NavRequest::Unlock while not locked");
                NavEvent::NoChanges { from, request }
            }
        }
    }
}

/// Replaces [`MenuBuilder`]s with proper [`TreeMenu`]s.
pub(crate) fn insert_tree_menus(
    mut commands: Commands,
    builders: Query<(Entity, &MenuBuilder), With<MenuSetting>>,
    queries: NavQueries,
) {
    use FocusState::{Active, Focused, Prioritized};
    let mut inserts = Vec::new();
    let no_focus_msg = "Within a menu built with MenuBuilder, there must be at least one entity \
         with the Focusable component, none were found";
    for (entity, builder) in &builders {
        let children = queries.children.focusables_of(entity);
        let child = children
            .iter()
            .find_map(|e| {
                let (_, focusable) = queries.focusables.get(*e).ok()?;
                matches!(focusable.state, Prioritized | Active | Focused).then_some(e)
            })
            .unwrap_or_else(|| children.first().expect(no_focus_msg));
        if let Ok(focus_parent) = builder.try_into() {
            let menu = TreeMenu {
                focus_parent,
                active_child: *child,
            };
            inserts.push((entity, (menu,)));
        } else {
            warn!("Encountered a non-translated named menu builder");
        }
        commands.entity(entity).remove::<MenuBuilder>();
    }
    commands.insert_or_spawn_batch(inserts);
}

/// System to set the first [`Focusable`] to [`FocusState::Focused`]
/// when no navigation has been done yet.
///
/// This also sets `Active` state and `active_child` of menus leading
/// to the current focusable.
pub(crate) fn set_first_focused(
    has_focused: Query<(), With<Focused>>,
    mut queries: ParamSet<(NavQueries, MutQueries)>,
    mut events: EventWriter<NavEvent>,
) {
    if has_focused.is_empty() {
        if let Some(to_focus) = queries.p0().pick_first_focused() {
            let breadcrumb = queries.p0().root_path(to_focus);
            queries.p1().update_focus(&[], &breadcrumb);
            events.send(NavEvent::InitiallyFocused(to_focus));
        }
    }
}

pub(crate) fn consistent_menu(
    updated_focusables: Query<(Entity, &Focusable), Changed<Focusable>>,
    children: ChildQueries,
    mut menus: Query<(Entity, &mut TreeMenu)>,
) {
    for (entity, updated) in &updated_focusables {
        if updated.state() != FocusState::Blocked {
            continue;
        }
        for (menu_entity, mut menu) in &mut menus {
            if menu.active_child != entity {
                continue;
            }
            if let Some(new_active) = children.focusables_of(menu_entity).first().copied() {
                menu.active_child = new_active;
            }
            // We found the unique menu that leads to the changed entity
            // continue to check for next changed focusable.
            break;
        }
    }
}

/// Listen to [`NavRequest`] and update the state of [`Focusable`] entities
/// when relevant.
pub(crate) fn listen_nav_requests<STGY: SystemParam>(
    mut queries: ParamSet<(NavQueries, MutQueries)>,
    mquery: StaticSystemParam<STGY>,
    mut lock: ResMut<NavLock>,
    mut requests: EventReader<NavRequest>,
    mut events: EventWriter<NavEvent>,
) where
    for<'w, 's> SystemParamItem<'w, 's, STGY>: MenuNavigationStrategy,
{
    let no_focused = "Tried to execute a NavRequest \
            when no focusables exist, \
            NavRequest does nothing if \
            there isn't any navigation to do.";

    // Cache focus result from previous iteration to avoid re-running costly `pick_first_focused`
    let mut computed_focused = None;
    for request in requests.iter() {
        if lock.is_locked() && *request != NavRequest::Unlock {
            continue;
        }
        // We use `pick_first_focused` instead of `Focused` component for first
        // iteration because `set_first_focused` just before `listen_nav_request`
        // without a command flush in-between.
        let picked = || queries.p0().pick_first_focused();
        let focused = match computed_focused.or_else(picked) {
            Some(focused) => focused,
            None => {
                warn!(no_focused);
                return;
            }
        };
        let from = Vec::new();
        let event = resolve(focused, *request, &queries.p0(), &mut lock, from, &*mquery);
        if let NavEvent::FocusChanged { to, from } = &event {
            computed_focused = Some(queries.p1().update_focus(from, to));
        };
        events.send(event);
    }
}

/// The child [`TreeMenu`] of `focusable`.
fn child_menu<'a>(
    focusable: Entity,
    queries: &'a NavQueries,
) -> Option<(Entity, &'a TreeMenu, &'a MenuSetting)> {
    queries
        .menus
        .iter()
        .find(|e| e.1.focus_parent == Some(focusable))
}

/// The [`TreeMenu`] containing `focusable`, if any.
pub(crate) fn parent_menu(
    focusable: Entity,
    queries: &NavQueries,
) -> Option<(Entity, TreeMenu, MenuSetting)> {
    let parent = queries.parents.get(focusable).ok()?.get();
    match queries.menus.get(parent) {
        Ok((_, tree, setting)) => Some((parent, tree.clone(), *setting)),
        Err(_) => parent_menu(parent, queries),
    }
}

impl<'w, 's> ChildQueries<'w, 's> {
    /// All sibling [`Focusable`]s within a single [`TreeMenu`].
    pub(crate) fn focusables_of(&self, menu: Entity) -> Vec<Entity> {
        use FocusState::Blocked;
        let is_focusable = |e: &&_| {
            self.is_focusable
                .get(**e)
                .map_or(false, |f| f.state != Blocked)
        };
        match self.children.get(menu) {
            Ok(direct_children) => {
                let focusables = direct_children.iter().filter(is_focusable).cloned();
                let transitive_focusables = direct_children
                    .iter()
                    .filter(|e| !self.is_focusable.contains(**e))
                    .filter(|e| !self.is_menu.contains(**e))
                    .flat_map(|e| self.focusables_of(*e));
                focusables.chain(transitive_focusables).collect()
            }
            Err(_) => Vec::new(),
        }
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

/// Navigate downward the menu hierarchy, traversing all prioritized children.
fn focus_deep<'a>(mut menu: &'a TreeMenu, queries: &'a NavQueries) -> Vec<Entity> {
    let mut ret = Vec::with_capacity(4);
    loop {
        let last = menu.active_child;
        ret.insert(0, last);
        menu = match child_menu(last, queries) {
            Some((_, menu, _)) => menu,
            None => return ret,
        };
    }
}

/// Cycle through a [scoped menu](MenuSetting::scope) according to menu settings.
///
/// Returns the index of the element to focus according to `direction`.
/// Cycles if `cycles` and goes over `max_value` or goes bellow 0.
/// `None` if the direction is a dead end.
fn resolve_index(
    from: usize,
    cycles: bool,
    direction: events::ScopeDirection,
    max_value: usize,
) -> Option<usize> {
    use events::ScopeDirection::*;
    match (direction, from) {
        (Previous, 0) => cycles.then_some(max_value),
        (Previous, from) => Some(from - 1),
        (Next, from) if from == max_value => cycles.then_some(0),
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
