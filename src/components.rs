//! Focusable components and bundles to ease navigable UI declaration.
use bevy::prelude::{Bundle, ButtonBundle, Component, NodeBundle};

use crate::{
    menu::{MenuBuilder, MenuSetting, NavMarker},
    resolve::Focusable,
};

/// A button like the default bevy [`ButtonBundle`], but with an added
/// [`Focusable`] component so that it can be used with this crate.
#[derive(Default, Clone, Bundle)]
pub struct FocusableButtonBundle {
    /// The bevy components.
    pub button_bundle: ButtonBundle,
    /// The [`Focusable`] type.
    pub focus: Focusable,
}
impl From<ButtonBundle> for FocusableButtonBundle {
    fn from(button_bundle: ButtonBundle) -> Self {
        FocusableButtonBundle {
            button_bundle,
            ..Default::default()
        }
    }
}

/// A [`NodeBundle`] delimiting a menu,
/// which [`Focusable`] will be marked with `marker`.
///
/// - See [`MenuSetting`] for details on how menus work.
/// - See [`NavMarker`] for how marking works.
#[derive(Bundle)]
pub struct MarkingMenuBundle<T: Component> {
    /// How navigation within that menu works.
    pub setting: MenuSetting,
    /// Specify from where this menu is reachable.
    pub builder: MenuBuilder,
    /// What component of type `T` to add to all [`Focusable`]s within
    /// this menu.
    pub marker: NavMarker<T>,
    /// The bevy components.
    pub node: NodeBundle,
}
/// A [`NodeBundle`] delimiting a menu.
///
/// - See [`MenuSetting`] for details on how menus work.
/// - Use [`MenuBundle::marking`] if you need [`Focusable`]s in your menu to
///   share a specific component.
#[derive(Clone, Bundle)]
pub struct MenuBundle {
    /// How navigation within that menu works.
    pub setting: MenuSetting,
    /// Specify from where this menu is reachable.
    pub builder: MenuBuilder,
    /// The bevy components.
    pub node: NodeBundle,
}
impl MenuBundle {
    /// Add `marker` to all [`Focusable`]s in this menu whenever it is created.
    ///
    /// See [`NavMarker`] for how marking works.
    pub fn marking<T: Component>(self, marker: T) -> MarkingMenuBundle<T> {
        let Self {
            setting,
            builder,
            node,
        } = self;
        MarkingMenuBundle {
            setting,
            builder,
            marker: NavMarker(marker),
            node,
        }
    }
}
