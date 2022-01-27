//! Declare menu navigation through labels.
//!
//! The most difficult part of the API to deal with is giving
//! [`NavMenu::reachable_from`] the `Entity` for of the button used to reach
//! it.
//!
//! This forces you to divide the whole menu construction in multiple
//! parts and keep track of intermediary values if you want to make multple menus.
//!
//! *By-name declaration* let you simply add a label to your `Focusable` and refer
//! to it in [`NavMenu::reachable_from_named`]. The runtime then detects
//! labelled stuff and replace the partial [`NavMenu`] will the full one with the
//! proper entity id reference. This saves you from pre-spawning your buttons so
//! that you can associate their `id` with the proper submenu.
use std::borrow::Cow;

use bevy::prelude::*;

use crate::{marker::NavMarker, Focusable, NavMenu};

/// Component to specify creation of a [`NavMenu`] refering to their parent
/// focusable by [`Name`](https://docs.rs/bevy/latest/bevy/core/struct.Name.html)
/// and can mark their children focusables with `T`.
///
/// This is useful if, for example, you just want to spawn your UI without
/// keeping track of entity ids of your UI widgets.
#[derive(Bundle)]
pub struct NamedParentMarkingNavMenu<T: Send + Sync + 'static> {
    menu: NamedParentNavMenu,
    marker: NavMarker<T>,
}
impl<T: Component + Clone + Send + Sync + 'static> NamedParentMarkingNavMenu<T> {
    /// Create a `NamedParentMarkingNavMenu` with parent
    /// [named](https://docs.rs/bevy/latest/bevy/core/struct.Name.html)
    /// `parent_label` and marking children with `marker`.
    pub fn new(menu: NavMenu, marker: T, parent_label: impl Into<Cow<'static, str>>) -> Self {
        NamedParentMarkingNavMenu {
            menu: NamedParentNavMenu {
                menu,
                parent_label: Name::new(parent_label),
            },
            marker: NavMarker(marker),
        }
    }
}

/// Component to specify creation of a [`NavMenu`] refering to their parent
/// focusable by [`Name`](https://docs.rs/bevy/latest/bevy/core/struct.Name.html)
///
/// This is useful if, for example, you just want to spawn your UI without
/// keeping track of entity ids of your UI widgets.
#[derive(Component, Clone)]
pub struct NamedParentNavMenu {
    menu: NavMenu,
    parent_label: Name,
}
impl NamedParentNavMenu {
    /// Create a `NamedParentNavMenu` with parent
    /// [named](https://docs.rs/bevy/latest/bevy/core/struct.Name.html)
    /// `parent_label`
    pub fn new(menu: NavMenu, parent_label: impl Into<Cow<'static, str>>) -> Self {
        NamedParentNavMenu {
            menu,
            parent_label: Name::new(parent_label),
        }
    }
}

pub(crate) fn resolve_navmenu_label(
    mut cmds: Commands,
    unresolved: Query<(Entity, &NamedParentNavMenu)>,
    named: Query<(Entity, &Name), With<Focusable>>,
) {
    let mut to_insert = Vec::new();
    for (entity, labelled) in unresolved.iter() {
        let menu = match named.iter().find(|(_, n)| **n == labelled.parent_label) {
            Some((focus_parent, _)) => {
                let mut target = labelled.menu.clone();
                target.focus_parent = Some(focus_parent);
                cmds.entity(entity).remove::<NamedParentNavMenu>();
                target
            }
            None => {
                let name = labelled.parent_label.as_str();
                bevy::log::warn!(
                    "Tried to spawn a `NavMenu` with parent focusable {name}, but no\
                     `Focusable` has a `Name` component with that value."
                );
                continue;
            }
        };
        to_insert.push((entity, (menu,)));
    }
    cmds.insert_or_spawn_batch(to_insert);
}
