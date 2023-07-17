//! Definitions for [`cuicui_dsl`]'s `dsl!` macro.
use std::borrow::Cow;

use bevy::{ecs::system::EntityCommands, prelude::*};
use cuicui_dsl::DslBundle;

use crate::prelude::{Focusable, MenuBuilder, MenuSetting};

#[derive(Default, Debug)]
struct MenuData {
    setting: MenuSetting,
    reachable_from: Option<Cow<'static, str>>,
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
    focusable: bool,
}

impl<C> NavigationDsl<C> {
    /// Mark this node as a menu [reachable from] `reachable_from`
    ///
    /// [reachable from]: MenuBuilder::NamedParent
    pub fn menu(&mut self, reachable_from: impl Into<Cow<'static, str>>) {
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
        self.focusable = true;
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
}
impl<C: DslBundle> DslBundle for NavigationDsl<C> {
    fn insert(&mut self, cmds: &mut EntityCommands) -> Entity {
        self.inner.insert(cmds);
        if let Some(menu) = self.menu.take() {
            let builder = match menu.reachable_from {
                Some(menu) => MenuBuilder::NamedParent(Name::new(menu)),
                None => MenuBuilder::Root,
            };
            cmds.insert((menu.setting, builder));
        } else if self.focusable {
            cmds.insert(Focusable::default());
        }
        cmds.id()
    }
}
