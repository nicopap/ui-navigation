/*!
[`ButtonBundle`]: bevy::prelude::ButtonBundle
[Changed]: bevy::prelude::Changed
[doc-root]: ./index.html
[`Entity`]: bevy::prelude::Entity
[entity-id]: bevy::ecs::system::EntityCommands::id
[`FocusableButtonBundle`]: components::FocusableButtonBundle
[`Focusable::cancel`]: resolve::Focusable::cancel
[`Focusable::block`]: resolve::Focusable::block
[`Focusable::dormant`]: resolve::Focusable::dormant
[`Focusable`]: resolve::Focusable
[`Focusable::lock`]: resolve::Focusable::lock
[`generic_default_mouse_input`]: systems::generic_default_mouse_input
[`InputMapping`]: systems::InputMapping
[`InputMapping::keyboard_navigation`]: systems::InputMapping::keyboard_navigation
[module-event_helpers]: events::NavEventReaderExt
[module-marking]: mark
[module-systems]: systems
[Name]: bevy::core::Name
[`NavEvent::FocusChanged`]: events::NavEvent::FocusChanged
[`NavEvent`]: events::NavEvent
[`NavEvent::InitiallyFocused`]: events::NavEvent::InitiallyFocused
[`MenuSetting`]: menu::MenuSetting
[`NavMenu`]: menu::MenuSetting
[`MenuBuilder`]: menu::MenuBuilder
[MenuBuilder::reachable_from]: menu::MenuBuilder::EntityParent
[MenuBuilder::reachable_from_named]: menu::MenuBuilder::from_named
[`NavRequest`]: events::NavRequest
[`NavRequest::Action`]: events::NavRequest::Action
[`NavRequest::FocusOn`]: events::NavRequest::FocusOn
[`NavRequest::Free`]: events::NavRequest::Unlock
[`NavRequest::Unlock`]: events::NavRequest::Unlock
[`NavRequest::ScopeMove`]: events::NavRequest::ScopeMove
[`NavRequestSystem`]: NavRequestSystem
*/
#![doc = include_str!("../Readme.md")]
mod commands;
#[cfg(feature = "bevy_ui")]
pub mod components;
pub mod events;
mod marker;
pub mod menu;
mod named;
mod resolve;
pub mod systems;

use std::marker::PhantomData;

use bevy::app::prelude::*;
use bevy::ecs::{
    prelude::Component,
    schedule::{ParallelSystemDescriptorCoercion, SystemLabel},
    system::{SystemParam, SystemParamItem},
};

pub use non_empty_vec::NonEmpty;

#[cfg(feature = "bevy_ui")]
use resolve::UiProjectionQuery;

/// Default imports for `bevy_ui_navigation`.
pub mod prelude {
    pub use crate::events::{NavEvent, NavEventReaderExt, NavRequest};
    pub use crate::menu::{MenuBuilder, MenuSetting};
    pub use crate::resolve::{
        FocusAction, FocusState, Focusable, Focused, MenuNavigationStrategy, NavLock,
    };
    pub use crate::NavRequestSystem;
    #[cfg(feature = "bevy_ui")]
    pub use crate::{DefaultNavigationPlugins, NavigationPlugin};
}
/// Utilities to mark focusables within a menu with a specific component.
pub mod mark {
    pub use crate::menu::NavMarker;
    pub use crate::NavMarkerPropagationPlugin;
}
/// Types useful to define your own custom navigation inputs.
pub mod custom {
    #[cfg(feature = "bevy_ui")]
    pub use crate::resolve::UiProjectionQuery;
    pub use crate::resolve::{Rect, ScreenBoundaries};
    pub use crate::GenericNavigationPlugin;
}

/// Plugin for menu marker propagation.
///
/// For a marker of type `T` to be propagated when using
/// [`mark::NavMarker`], you need to add a
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
/// while systems that emit [`NavRequest`] should run _before_ it.
/// For example, an input system should run before the `NavRequestSystem`.
///
/// Failing to do so won't cause logical errors, but will make the UI feel more slugish
/// than necessary. This is especially critical of you are running on low framerate.
///
/// # Example
///
/// ```rust, no_run
/// use bevy_ui_navigation::prelude::*;
/// use bevy_ui_navigation::events::Direction;
/// use bevy_ui_navigation::custom::GenericNavigationPlugin;
/// use bevy::prelude::*;
/// # use std::marker::PhantomData;
/// # use bevy::ecs::system::SystemParam;
/// # #[derive(SystemParam)] struct MoveCursor3d<'w, 's> {
/// #   #[system_param(ignore)] _foo: PhantomData<(&'w (), &'s ())>
/// # }
/// # impl<'w, 's> MenuNavigationStrategy for MoveCursor3d<'w, 's> {
/// #   fn resolve_2d<'a>(
/// #       &self,
/// #       focused: Entity,
/// #       direction: Direction,
/// #       cycles: bool,
/// #       siblings: &'a [Entity],
/// #   ) -> Option<&'a Entity> { None }
/// # }
/// # fn button_system() {}
/// fn main() {
///     App::new()
///         .add_plugin(GenericNavigationPlugin::<MoveCursor3d>::new())
///         // ...
///         // Add the button color update system after the focus update system
///         .add_system(button_system.after(NavRequestSystem))
///         // ...
///         .run();
/// }
/// ```
///
/// [`NavRequest`]: prelude::NavRequest
/// [`NavEvent`]: prelude::NavEvent
/// [`Focusable`]: prelude::Focusable
#[derive(Clone, Debug, Hash, PartialEq, Eq, SystemLabel)]
pub struct NavRequestSystem;

/// The navigation plugin.
///
/// Add it to your app with `.add_plugin(NavigationPlugin::new())` and send
/// [`NavRequest`]s to move focus within declared [`Focusable`] entities.
///
/// You should prefer `bevy_ui` provided defaults
/// if you don't want to bother with that.
///
/// # Note on generic parameters
///
/// The `STGY` type parameter might seem complicated, but all you have to do
/// is for your type to implement [`SystemParam`] and [`MenuNavigationStrategy`].
///
/// [`MenuNavigationStrategy`]: resolve::MenuNavigationStrategy
/// [`Focusable`]: prelude::Focusable
/// [`NavRequest`]: prelude::NavRequest
#[derive(Default)]
pub struct GenericNavigationPlugin<STGY>(PhantomData<fn() -> STGY>);
#[cfg(feature = "bevy_ui")]
pub type NavigationPlugin<'w, 's> = GenericNavigationPlugin<UiProjectionQuery<'w, 's>>;

impl<STGY: resolve::MenuNavigationStrategy> GenericNavigationPlugin<STGY> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}
impl<STGY: SystemParam + 'static> Plugin for GenericNavigationPlugin<STGY>
where
    for<'w, 's> SystemParamItem<'w, 's, STGY>: resolve::MenuNavigationStrategy,
{
    fn build(&self, app: &mut App) {
        app.add_event::<events::NavRequest>()
            .add_event::<events::NavEvent>()
            .insert_resource(resolve::NavLock::new())
            .add_system(resolve::set_first_focused.before(NavRequestSystem))
            .add_system(resolve::consistent_menu.before(NavRequestSystem))
            .add_system(resolve::listen_nav_requests::<STGY>.label(NavRequestSystem))
            // PostUpdate because we want the Menus to be setup correctly before the
            // next call to `set_first_focused`, which depends on the Menu tree layout
            // existing already to chose a "intuitively correct" first focusable.
            // The user is most likely to spawn his UI in the Update stage, so it makes
            // sense to react to changes in the PostUpdate stage.
            .add_system_to_stage(
                CoreStage::PostUpdate,
                named::resolve_named_menus.before(resolve::insert_tree_menus),
            )
            .add_system_to_stage(CoreStage::PostUpdate, resolve::insert_tree_menus);
    }
}

/// The navigation plugin and the default input scheme.
///
/// Add it to your app with `.add_plugins(DefaultNavigationPlugins)`.
///
/// This provides default implementations for input handling, if you want
/// your own custom input handling, you should use [`NavigationPlugin`] and
/// provide your own input handling systems.
#[cfg(feature = "bevy_ui")]
pub struct DefaultNavigationPlugins;
#[cfg(feature = "bevy_ui")]
impl PluginGroup for DefaultNavigationPlugins {
    fn build(&mut self, group: &mut bevy::app::PluginGroupBuilder) {
        group.add(NavigationPlugin::new());
        group.add(systems::DefaultNavigationSystems);
    }
}
