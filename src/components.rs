//! Focusable components and bundles to ease navigable UI declaration.
#![allow(clippy::forget_non_drop)]
use bevy::prelude::{Bundle, ButtonBundle};

use super::Focusable;

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
