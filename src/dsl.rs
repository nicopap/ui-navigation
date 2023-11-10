//! Definitions for [`cuicui_dsl`]'s `dsl!` macro.
use std::borrow::Cow;

use bevy::prelude::*;
use cuicui_dsl::{DslBundle, EntityCommands};

use crate::prelude::{FocusAction, Focusable, MenuBuilder, MenuSetting};

#[derive(Default, Debug)]
struct MenuData {
    setting: MenuSetting,
    reachable_from: Option<Cow<'static, str>>,
}

#[derive(Default, Debug, Copy, Clone)]
enum DslState {
    #[default]
    Normal,
    Blocked,
    Priority,
}

/// The [`DslBundle`] for menu navigation.
///
/// - Use [`Self::menu`] to mark a node as a menu
///   - Use [`Self::menu_root`] to mark a node as the [root menu](MenuBuilder::Root)
///   - Use [`Self::scope`] to make the menu a [scope menu](MenuSetting::scope)
///   - Use [`Self::wrap`] to make the menu [wrapping](MenuSetting::wrapping)
/// - Use [`Self::focus`] to mark a node as focusable
#[derive(Default, Debug, Deref, DerefMut)]
pub struct NavigationDsl<C = ()> {
    #[deref]
    inner: C,
    menu: Option<MenuData>,
    focusable: Option<(FocusAction, DslState)>,
}

#[cfg_attr(feature = "cuicui_chirp", cuicui_chirp::parse_dsl_impl(delegate = inner))]
impl<C> NavigationDsl<C> {
    /// Similar to [`Self::menu_cow`], but allocates `&str`.
    pub fn menu(&mut self, reachable_from: &str) {
        self.menu_cow(reachable_from.to_string());
    }

    /// Mark this node as a menu [reachable from] `reachable_from`
    ///
    /// [reachable from]: MenuBuilder::NamedParent
    #[cfg_attr(feature = "cuicui_chirp", parse_dsl(ignore))]
    pub fn menu_cow(&mut self, reachable_from: impl Into<Cow<'static, str>>) {
        let menu = self.menu.get_or_insert(default());
        menu.reachable_from = Some(reachable_from.into());
    }
    /// Mark this node as a [scope menu](MenuSetting::scope).
    pub fn scope(&mut self) {
        let menu = self.menu.get_or_insert(default());
        menu.setting.scope = true;
    }
    /// Mark this node as a [`Focusable`].
    ///
    /// This is incompatible with the menu-based methods, if both are used,
    /// menu prevails.
    pub fn focus(&mut self) {
        self.focusable = Some(default());
    }
    /// Mark this node as the [root menu](MenuBuilder::Root).
    pub fn menu_root(&mut self) {
        let menu = self.menu.get_or_insert(default());
        menu.reachable_from = None;
    }
    /// Mark this menu as [wrapping](MenuSetting::wrapping).
    pub fn wrap(&mut self) {
        let menu = self.menu.get_or_insert(default());
        menu.setting.wrapping = true;
    }
    /// Set the [`FocusAction`] for this focusable.
    pub fn action(&mut self, action: FocusAction) {
        let (current_action, _) = self.focusable.get_or_insert(default());
        *current_action = action;
    }
    /// Set this focusable as [prioritized](Focusable::prioritized).
    pub fn prioritized(&mut self) {
        let (_, state) = self.focusable.get_or_insert(default());
        *state = DslState::Priority;
    }
    /// Set this focusable as [blocked](Focusable::blocked).
    pub fn blocked(&mut self) {
        let (_, state) = self.focusable.get_or_insert(default());
        *state = DslState::Blocked;
    }
}
impl<C: DslBundle> DslBundle for NavigationDsl<C> {
    fn insert(&mut self, cmds: &mut EntityCommands) {
        self.inner.insert(cmds);
        if let Some(menu) = self.menu.take() {
            let builder = match menu.reachable_from {
                Some(menu) => MenuBuilder::NamedParent(Name::new(menu)),
                None => MenuBuilder::Root,
            };
            cmds.insert((menu.setting, builder));
        } else if let Some((action, state)) = self.focusable {
            let focusable = match action {
                FocusAction::Normal => Focusable::new(),
                FocusAction::Cancel => Focusable::cancel(),
                FocusAction::Lock => Focusable::lock(),
            };
            let focusable = match state {
                DslState::Normal => focusable,
                DslState::Blocked => focusable.blocked(),
                DslState::Priority => focusable.prioritized(),
            };
            cmds.insert(focusable);
        }
    }
}
