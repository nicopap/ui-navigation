use bevy::ecs::{entity::Entity, system::Command};

use crate::{FocusState, Focusable, Focused};

pub(crate) fn set_focus_state(entity: Entity, new_state: FocusState) -> UpdateFocusable {
    UpdateFocusable { entity, new_state }
}
pub(crate) struct UpdateFocusable {
    entity: Entity,
    new_state: FocusState,
}
impl Command for UpdateFocusable {
    fn write(self, world: &mut bevy::prelude::World) {
        let mut entity = world.entity_mut(self.entity);
        if let Some(mut entity) = entity.get_mut::<Focusable>() {
            entity.focus_state = self.new_state;
        }
        match self.new_state {
            FocusState::Focused => {
                entity.insert(Focused);
            }
            _ => {
                entity.remove::<Focused>();
            }
        };
    }
}
