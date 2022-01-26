//! Automatic menu marker component propagation.
//!
//! Enables user to specify their own marker to add to [`Focusable`]s within
//! [`NavMenu`]s.
use std::{iter, marker::PhantomData};

use bevy::prelude::*;

use crate::{Focusable, NavMenu};

/// Component to add to [`NavMenu`] entities to propagate `T` to all
/// [`Focusable`] children of that menu.
///
/// See the [`MarkingMenu`] constructor to create a `NavMarker`.
#[derive(Component)]
struct NavMarker<T>(T);

/// A [`NavMenu`] with automatic `T` marker propagation
///
/// A `NavMenu` created from this bundle will automatically mark all
/// [`Focusable`]s within that menu with the `T` component.
///
/// `T` must be `Component` because it will be added as component to
/// [`Focusable`] within this menu. It also needs to be `Clone`, because we
/// need to make a copy of it to add it as component to other entities.
///
/// In order for `T` to propagate to the children of this menu, you need
/// to add the [`NavMarkerPropagationPlugin<T>`] to your bevy app.
#[derive(Bundle)]
pub struct MarkingMenu<T: Send + Sync + 'static> {
    menu: NavMenu,
    marker: NavMarker<T>,
}
impl<T: Component + Clone + Send + Sync + 'static> MarkingMenu<T> {
    /// Prefer [`MarkingMenu::reachable_from`] and [`MarkingMenu::root`] to this
    ///
    /// `new` is useful to programmatically set the parent if you have an
    /// optional value. This saves you from a `match focus_parent`.
    ///
    /// `marker` is the component that will be added to all [`Focusable`]
    /// entities contained within this menu.
    pub fn new(focus_parent: Option<Entity>, marker: T) -> Self {
        MarkingMenu {
            menu: NavMenu::new(focus_parent),
            marker: NavMarker(marker),
        }
    }

    /// Set this menu as having no parents
    ///
    /// `marker` is the component that will be added to all [`Focusable`]
    /// entities contained within this menu.
    pub fn root(marker: T) -> Self {
        Self::new(None, marker)
    }

    /// Set this menu as closed (no cycling)
    pub fn closed(mut self) -> Self {
        self.menu = self.menu.closed();
        self
    }

    /// Set this menu as cycling
    ///
    /// ie: going left from the leftmost element goes to the rightmost element
    pub fn cycling(mut self) -> Self {
        self.menu = self.menu.cycling();
        self
    }

    /// Set this menu as a scope menu
    ///
    /// Meaning: controlled with [`NavRequest::ScopeMove`](crate::NavRequest::ScopeMove) even when the
    /// focused element is not in this menu, but in a submenu reachable from
    /// this one.
    pub fn scope(mut self) -> Self {
        self.menu = self.menu.scope();
        self
    }

    /// Set this menu as reachable from a given [`Focusable`]
    ///
    /// When requesting [`NavRequest::Action`](crate::NavRequest::Action)
    /// when `focusable` is focused, the focus will be changed to a
    /// focusable within this menu.
    ///
    /// `marker` is the component that will be added to all [`Focusable`]
    /// entities contained within this menu.
    ///
    /// # Important
    ///
    /// You must ensure this doesn't create a cycle. Eg: you shouldn't be able
    /// to reach `NavMenu` X from `Focusable` Y if there is a path from
    /// `NavMenu` X to `Focusable` Y.
    pub fn reachable_from(focusable: Entity, marker: T) -> Self {
        Self::new(Some(focusable), marker)
    }
}

#[allow(clippy::type_complexity)]
fn mark_menu_entries<T: Component + Clone>(
    mut cmds: Commands,
    new_markers: Query<(Entity, &NavMarker<T>), (Added<NavMarker<T>>, With<NavMenu>)>,
    queries: crate::NavQueries,
) {
    let mut to_insert = Vec::with_capacity(32);
    for (new_menu, marker) in new_markers.iter() {
        let repeat_marker = iter::repeat((marker.0.clone(),));
        let menu_children = crate::children_focusables(new_menu, &queries);
        to_insert.extend(menu_children.into_iter().zip(repeat_marker));
    }
    cmds.insert_or_spawn_batch(to_insert);
}
fn mark_new_focusable<T: Component + Clone>(
    mut cmds: Commands,
    new_focusables: Query<Entity, Added<Focusable>>,
    markers: Query<&NavMarker<T>>,
    queries: crate::NavQueries,
) {
    let mut to_insert = Vec::with_capacity(32);
    for new_focusable in new_focusables.iter() {
        let containing_menu = match crate::parent_menu(new_focusable, &queries) {
            Some((c, _)) => c,
            None => continue,
        };
        let marker = match markers.get(containing_menu) {
            Ok(m) => m.0.clone(),
            Err(_) => continue,
        };
        to_insert.push((new_focusable, (marker,)));
    }
    cmds.insert_or_spawn_batch(to_insert);
}

/// Plugin for menu marker propagation.
///
/// For a marker of type `T` to be propagated when using [`MarkingMenu`], you
/// need to add a `NavMarkerPropagationPlugin<T>` to your bevy app. It is
/// possible to add any amount of `NavMarkerPropagationPlugin<T>` for as many
/// `T` you need to propagate through the menu system.
pub struct NavMarkerPropagationPlugin<T>(PhantomData<T>);
impl<T> NavMarkerPropagationPlugin<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        NavMarkerPropagationPlugin(PhantomData)
    }
}

impl<T: 'static + Sync + Send + Component + Clone> Plugin for NavMarkerPropagationPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_system(mark_menu_entries::<T>)
            .add_system(mark_new_focusable::<T>);
    }
}
