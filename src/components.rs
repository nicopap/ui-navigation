//! Focusable components and bundles to ease navigable UI declaration.
use bevy::prelude::{Bundle, ButtonBundle, Component, NodeBundle};

use crate::{
    menu::{MenuBuilder, MenuSetting, NavMarker},
    resolve::Focusable,
};

#[derive(Default, Clone, Bundle)]
pub struct FocusableButtonBundle {
    #[bundle]
    pub button_bundle: ButtonBundle,
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

#[derive(Bundle)]
pub struct MarkingMenuBundle<T: Component> {
    pub setting: MenuSetting,
    pub builder: MenuBuilder,
    pub marker: NavMarker<T>,
    #[bundle]
    pub node: NodeBundle,
}
#[derive(Clone, Bundle)]
pub struct MenuBundle {
    pub setting: MenuSetting,
    pub builder: MenuBuilder,
    #[bundle]
    pub node: NodeBundle,
}
impl MenuBundle {
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
