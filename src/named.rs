//! Declare menu navigation through [`Name`].
//!
//! The most difficult part of the API when creating UI scenes,
//! was using [`MenuBuilder::EntityParent`],
//! providing the [`Entity`] for the [`Focusable`] the menu is reachable from,
//! forced users to separate and order the creation of their menus.
//!
//! *By-name declaration* let you simply add a [`Name`] to your [`Focusable`]
//! and refer to it in [`MenuBuilder::NamedParent`].
//!
//! The runtime then detects labelled stuff
//! and replace the partial [`MenuBuilder`]
//! with the full [`TreeMenu`] with the proper entity id reference.
//! This saves you from pre-spawning your buttons
//! so that you can associate their `id` with the proper submenu.
//!
//! [`TreeMenu`]: crate::resolve::TreeMenu
use std::mem;

use bevy::core::Name;
use bevy::ecs::prelude::*;
use bevy::log::{debug, warn};
use bevy::time::Time;

use crate::{menu::MenuBuilder, resolve::Focusable};

pub(crate) fn resolve_named_menus(
    mut unresolved: Query<(Entity, &mut MenuBuilder)>,
    named: Query<(Entity, &Name), With<Focusable>>,
    time: Option<Res<Time>>,
) {
    use MenuBuilder::{EntityParent, NamedParent, Root};
    let each_second = || {
        let Some(time) = &time else { return true };
        time.elapsed_seconds_f64().fract() < time.delta_seconds_f64()
    };
    for (entity, mut builder) in &mut unresolved {
        let parent_name = match &mut *builder {
            NamedParent(name) => mem::take(name),
            // Already resolved / do not need to resolve name
            EntityParent(_) | Root => continue,
        };
        let with_parent_name = |(e, n)| (&parent_name == n).then_some(e);
        match named.iter().find_map(with_parent_name) {
            Some(focus_parent) => {
                debug!("Found parent focusable with name '{parent_name}' for menu {entity:?}");
                *builder = MenuBuilder::EntityParent(focus_parent);
            }
            None if each_second() => {
                warn!(
                    "Tried to spawn menu {entity:?} with parent focusable \
                    '{parent_name}', but no Focusable has a Name component \
                    with that value."
                );
                *builder = NamedParent(parent_name);
            }
            None => {
                *builder = NamedParent(parent_name);
            }
        }
    }
}
