#![doc = include_str!("../Readme.md")]
mod commands;
#[cfg(feature = "bevy-ui")]
pub mod components;
pub mod events;
mod marker;
mod named;
mod resolve;
mod seeds;
pub mod systems;

use std::marker::PhantomData;

use bevy::prelude::*;

pub use non_empty_vec::NonEmpty;

pub use seeds::NavMenu;

/// The [`Bundle`](https://docs.rs/bevy/0.7.0/bevy/ecs/bundle/trait.Bundle.html)s
/// returned by the [`NavMenu`] methods.
pub mod bundles {
    pub use crate::seeds::{MarkingMenuSeed, MenuSeed, NamedMarkingMenuSeed, NamedMenuSeed};
}
pub use events::{NavEvent, NavRequest};
pub use resolve::{FocusAction, FocusState, Focusable, Focused, NavLock};

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
#[derive(Clone, Debug, Hash, PartialEq, Eq, SystemLabel)]
pub struct NavRequestSystem;

/// The navigation plugin.
///
/// Add it to your app with `.add_plugin(NavigationPlugin)` and send
/// [`NavRequest`]s to move focus within declared [`Focusable`] entities.
pub struct NavigationPlugin;
impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NavRequest>()
            .add_event::<NavEvent>()
            .insert_resource(NavLock::new())
            .add_system(resolve::listen_nav_requests.label(NavRequestSystem))
            .add_system(resolve::set_first_focused)
            .add_system(resolve::insert_tree_menus)
            .add_system(named::resolve_named_menus.before(resolve::insert_tree_menus));
    }
}
