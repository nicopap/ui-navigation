pub(crate) struct ChildrenQueries<'w, 's, 'a> {
    children: &'a Query<'w, 's, &'static Children>,
    is_focusable: &'a Query<'w, 's, Entity, (With<Focusable>, Without<TreeMenu>)>,
    is_menu: &'a Query<'w, 's, Entity, Or<(With<TreeMenu>, With<seeds::TreeMenuSeed>)>>,
}
#[allow(clippy::type_complexity)]
#[derive(SystemParam)]
pub(crate) struct NavQueries<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static Parent>,
    focusables: Query<'w, 's, (Entity, &'static mut Focusable), Without<TreeMenu>>,
    menus: Query<'w, 's, (Entity, &'static mut TreeMenu), Without<Focusable>>,
    transform: Query<'w, 's, &'static GlobalTransform>,
    is_focusable: Query<'w, 's, Entity, (With<Focusable>, Without<TreeMenu>)>,
    is_menu: Query<'w, 's, Entity, Or<(With<TreeMenu>, With<seeds::TreeMenuSeed>)>>,
}
impl<'w, 's> NavQueries<'w, 's> {
    pub(crate) fn children(&self) -> ChildrenQueries<'w, 's, '_> {
        ChildrenQueries {
            children: &self.children,
            is_focusable: &self.is_focusable,
            is_menu: &self.is_menu,
        }
    }
    fn focused(&self) -> Option<Entity> {
        use FocusState::{Dormant, Focused};
        let menu_dormant = |menu: &TreeMenu| menu.focus_parent.is_none().then(|| menu.active_child);
        let any_dormant = |(e, focus): (Entity, &Focusable)| (focus.state == Dormant).then(|| e);
        let any_dormant = || self.focusables.iter().find_map(any_dormant);
        let root_dormant = || self.menus.iter().find_map(|(_, menu)| menu_dormant(menu));
        let fallback = || self.focusables.iter().next().map(|(entity, _)| entity);
        self.focusables
            .iter()
            .find_map(|(e, focus)| (focus.state == Focused).then(|| e))
            .or_else(root_dormant)
            .or_else(any_dormant)
            .or_else(fallback)
    }
    fn set_entity_focus(&mut self, cmds: &mut Commands, entity: Entity, state: FocusState) {
        if let Ok((_, mut focusable)) = self.focusables.get_mut(entity) {
            focusable.state = state;
            cmds.add(set_focus_state(entity, state));
        }
    }
}
