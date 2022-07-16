/*!
[`ButtonBundle`]: ButtonBundle
[Changed]: Changed
[doc-root]: ./index.html
[`Entity`]: Entity
[entity-id]: bevy::ecs::system::EntityCommands::id
[`FocusableButtonBundle`]: components::FocusableButtonBundle
[`Focusable::cancel`]: Focusable::cancel
[`Focusable::dormant`]: Focusable::dormant
[`Focusable`]: Focusable
[`Focusable::lock`]: Focusable::lock
[`generic_default_mouse_input`]: systems::generic_default_mouse_input
[`InputMapping`]: systems::InputMapping
[`InputMapping::keyboard_navigation`]: systems::InputMapping::keyboard_navigation
[module-event_helpers]: event_helpers
[module-marking]: bundles::MenuSeed
[module-systems]: systems
[Name]: Name
[`NavEvent::FocusChanged`]: NavEvent::FocusChanged
[`NavEvent`]: NavEvent
[`NavEvent::InitiallyFocused`]: NavEvent::InitiallyFocused
[`NavMenu`]: NavMenu
[NavMenu::reachable_from]: NavMenu::reachable_from
[NavMenu::reachable_from_named]: NavMenu::reachable_from_named
[`NavRequest::Action`]: NavRequest::Action
[`NavRequest::FocusOn`]: NavRequest::FocusOn
[`NavRequest::Free`]: NavRequest::Free
[`NavRequest::ScopeMove`]: NavRequest::ScopeMove
[`NavRequestSystem`]: NavRequestSystem
*/
#![doc = include_str!("../Readme.md")]
mod commands;
#[cfg(feature = "bevy_ui")]
pub mod components;
pub mod event_helpers;
pub mod events;
mod marker;
mod named;
mod resolve;
mod seeds;
pub mod systems;

use std::marker::PhantomData;

use bevy::{
    ecs::system::{SystemParam, SystemParamItem},
    prelude::*,
};

pub use events::{NavEvent, NavRequest};
pub use non_empty_vec::NonEmpty;
pub use resolve::{
    FocusAction, FocusState, Focusable, Focused, MoveParam, NavLock, Rect, ScreenBoundaries,
    UiProjectionQuery,
};
pub use seeds::NavMenu;
#[cfg(feature = "bevy_ui")]
pub use systems::DefaultNavigationSystems;

/// The [`Bundle`](bevy::prelude::Bundle)s
/// returned by the [`NavMenu`] methods.
pub mod bundles {
    pub use crate::seeds::{MarkingMenuSeed, MenuSeed, NamedMarkingMenuSeed, NamedMenuSeed};
}

/// Plugin for menu marker propagation.
///
/// For a marker of type `T` to be propagated when using
/// [`marking`](bundles::MenuSeed::marking), you need to add a
/// `NavMarkerPropagationPlugin<T>` to your bevy app. It is possible to add any
/// amount of `NavMarkerPropagationPlugin<T>` for as many `T` you need to
/// propagate through the menu system.
pub struct NavMarkerPropagationPlugin<T>(PhantomData<T>);
impl<T> NavMarkerPropagationPlugin<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        NavMarkerPropagationPlugin(PhantomData)
    }
}

impl<T: 'static + Sync + Send + Component + Clone> Plugin for NavMarkerPropagationPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_system(marker::mark_new_menus::<T>)
            .add_system(marker::mark_new_focusables::<T>);
    }
}

/// The label of the system in which the [`NavRequest`] events are handled, the
/// focus state of the [`Focusable`]s is updated and the [`NavEvent`] events
/// are sent.
///
/// Systems updating visuals of UI elements should run _after_ the `NavRequestSystem`,
/// while systems that emit [`NavRequest`] should run _before_ it. For example, the
/// [`systems::default_mouse_input`] systems should run before the `NavRequestSystem`.
///
/// Failing to do so won't cause logical errors, but will make the UI feel more slugish
/// than necessary. This is especially critical of you are running on low framerate.
///
/// # Example
///
/// ```rust, no_run
/// use bevy::prelude::*;
/// use bevy_ui_navigation::{NavRequestSystem, DefaultNavigationPlugins};
/// # fn button_system() {}
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_plugins(DefaultNavigationPlugins)
///         // ...
///         // Add the button color update system after the focus update system
///         .add_system(button_system.after(NavRequestSystem))
///         // ...
///         .run();
/// }
/// ```
#[derive(Clone, Debug, Hash, PartialEq, Eq, SystemLabel)]
pub struct NavRequestSystem;

/// The navigation plugin.
///
/// Add it to your app with `.add_plugin(NavigationPlugin)` and send
/// [`NavRequest`]s to move focus within declared [`Focusable`] entities.
///
/// This means you'll also have to add manaully the systems from [`systems`]
/// and [`systems::InputMapping`]. You should prefer [`DefaultNavigationPlugins`]
/// if you don't want to bother with that.
///
/// # Note on generic parameters
///
/// The `MP` type parameter might seem complicated, but all you have to do
/// is for your type to implement [`SystemParam`] and [`MoveParam`].
/// See the [`resolve::UiProjectionQuery`] source code for implementation hints.
pub struct GenericNavigationPlugin<MP>(PhantomData<MP>);
pub type NavigationPlugin<'w, 's> = GenericNavigationPlugin<UiProjectionQuery<'w, 's>>;
unsafe impl<T> Send for GenericNavigationPlugin<T> {}
unsafe impl<T> Sync for GenericNavigationPlugin<T> {}

impl<MP: MoveParam> GenericNavigationPlugin<MP> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}
impl<MP: SystemParam + 'static> Plugin for GenericNavigationPlugin<MP>
where
    for<'w, 's> SystemParamItem<'w, 's, MP>: MoveParam,
{
    fn build(&self, app: &mut App) {
        app.add_event::<NavRequest>()
            .add_event::<NavEvent>()
            .insert_resource(NavLock::new())
            .add_system(resolve::listen_nav_requests::<MP>.label(NavRequestSystem))
            .add_system(resolve::set_first_focused)
            .add_system(resolve::insert_tree_menus)
            .add_system(named::resolve_named_menus.before(resolve::insert_tree_menus));
    }
}
/// The navigation plugin and the default input scheme.
///
/// Add it to your app with `.add_plugins(DefaultNavigationPlugins)`.
///
/// This provides default implementations for input handling, if you want
/// your own custom input handling, you should use [`NavigationPlugin`] and
/// provide your own input handling systems.
pub struct DefaultNavigationPlugins;
impl PluginGroup for DefaultNavigationPlugins {
    fn build(&mut self, group: &mut bevy::app::PluginGroupBuilder) {
        group.add(GenericNavigationPlugin::<UiProjectionQuery>::new());
        #[cfg(feature = "bevy_ui")]
        group.add(DefaultNavigationSystems);
    }
}
