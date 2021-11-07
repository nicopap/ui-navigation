mod events;

use std::cmp::Ordering;

use bevy::ecs::system::{QuerySingleError, SystemParam};
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;

pub use crate::events::{Command as NavCommand, NavEvent};

#[derive(SystemParam)]
struct NavQueries<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static Parent>,
    focusables: Query<'w, 's, Entity, With<Focusable>>,
    nav_nodes: Query<'w, 's, (), With<NavNode>>,
    actives: Query<'w, 's, (), Or<(With<Active>, With<Focused>)>>,
    transform: Query<'w, 's, &'static GlobalTransform>,
}

#[derive(Component)]
pub struct NavNode;

#[derive(Component)]
pub struct Focusable;

// TODO: consider using bevy::prelude::Interaction
// or at least something more ergonomic you can use methods on
/// Currently prevent user from setting Focused and Active
///
/// We just assume the first Focusable is an `Active` (and by extension a
/// `Focused`, as currently there is no tree traversal) Then once we have a
/// `Focused` everything should work as expected.
#[derive(Component)]
#[non_exhaustive]
pub struct Focused;

#[derive(Component)]
#[non_exhaustive]
pub struct Active;

// Alternative design: Instead of evaluating at focus-change time the
// neighbores, somehow cache them
enum Direction {
    South,
    North,
    East,
    West,
}
impl Direction {
    fn is_in(&self, reference: Vec2, other: Vec2) -> bool {
        let coord = other - reference;
        use Direction::*;
        match self {
            South => coord.y < coord.x && coord.y < -coord.x,
            North => coord.y > coord.x && coord.y > -coord.x,
            East => coord.y < coord.x && coord.y > -coord.x,
            West => coord.y > coord.x && coord.y < -coord.x,
        }
    }
}
impl TryFrom<NavCommand> for Direction {
    type Error = ();
    fn try_from(value: NavCommand) -> Result<Self, Self::Error> {
        use Direction::*;
        use NavCommand::*;
        match value {
            MoveUp => Ok(North),
            MoveDown => Ok(South),
            MoveLeft => Ok(West),
            MoveRight => Ok(East),
            _ => Err(()),
        }
    }
}

fn move_focus_at(
    focused: Entity,
    direction: Direction,
    siblings: &[Entity],
    transform: &Query<&GlobalTransform>,
) -> Option<Entity> {
    let focused_loc = transform.get(focused).unwrap().translation.xy();
    siblings
        .iter()
        .filter(|sibling| {
            let sibling_loc = transform.get(**sibling).unwrap().translation;
            direction.is_in(focused_loc, sibling_loc.xy()) && **sibling != focused
        })
        .min_by(|s1, s2| {
            let s1_loc = transform.get(**s1).unwrap().translation;
            let s2_loc = transform.get(**s2).unwrap().translation;
            let s1_dist = focused_loc.distance_squared(s1_loc.xy());
            let s2_dist = focused_loc.distance_squared(s2_loc.xy());
            s1_dist.partial_cmp(&s2_dist).unwrap_or(Ordering::Equal)
        })
        .cloned()
}

fn change_focus_at(
    focused: Entity,
    command: NavCommand,
    siblings: &[Entity],
    transform: &Query<&GlobalTransform>,
) -> Option<Entity> {
    let direction: Direction = command.try_into().ok()?;
    move_focus_at(focused, direction, siblings, transform)
}

fn rootless_change_focus(
    focused: Entity,
    command: NavCommand,
    queries: &NavQueries,
    mut disactivated: Vec<Entity>,
) -> NavEvent {
    // In the case the user doesn't specify ANY NavNode, it's the most
    // simple case
    if queries.nav_nodes.is_empty() {
        let siblings: Vec<Entity> = queries.focusables.iter().collect();
        disactivated.push(focused);
        match change_focus_at(focused, command, &siblings, &queries.transform) {
            Some(to) => NavEvent::FocusChanged { to, disactivated },
            None => NavEvent::Uncaught { command, focused },
        }
    } else {
        // In case the user has specified AT LEAST one NavNode, we will act as
        // if there were no orphan Focusable (this may not be true, but it
        // would be very expensive to manage that case)
        NavEvent::Uncaught { command, focused }
    }
}

fn change_focus(
    focused: Entity,
    command: NavCommand,
    queries: &NavQueries,
    mut disactivated: Vec<Entity>,
) -> NavEvent {
    let nav_node = match containing_navnode(focused, queries) {
        Some(entity) => entity,
        None => return rootless_change_focus(focused, command, queries, disactivated),
    };
    let siblings = all_focusables(nav_node, queries);
    // TODO: better handling of missing Active/Focused
    let focused = get_active(&siblings, &queries.actives).unwrap_or(*siblings.first().unwrap());
    disactivated.push(focused);
    match change_focus_at(focused, command, &siblings, &queries.transform) {
        Some(to) => NavEvent::FocusChanged { to, disactivated },
        None => change_focus(nav_node, command, queries, disactivated),
    }
}
// Consideration: exploring a part of the UI graph every time a navigation
// request is sent might be too slow. Possible mitigation: caching parts of the
// navigation graph.
fn listen_nav_requests(
    focused: Query<Entity, With<Focused>>,
    mut events: EventReader<NavCommand>,
    queries: NavQueries,
    mut commands: Commands,
) {
    // TODO: this most likely breaks when there is more than a single event
    for command in events.iter() {
        let focused_id = focused.get_single().unwrap_or_else(|err| {
            if matches!(err, QuerySingleError::MultipleEntities(_)) {
                panic!("Multiple focused, not possible");
            }
            queries.focusables.iter().next().unwrap()
        });
        let change = change_focus(focused_id, *command, &queries, Vec::new());
        if let NavEvent::FocusChanged { to, disactivated } = change {
            for elem in disactivated {
                commands.entity(elem).remove::<Focused>();
                commands.entity(elem).remove::<Active>();
            }
            commands.entity(to).insert(Focused);
        }
    }
}

fn containing_navnode(focusable: Entity, queries: &NavQueries) -> Option<Entity> {
    match queries.parents.get(focusable).ok() {
        Some(Parent(parent)) if queries.nav_nodes.get(*parent).is_ok() => Some(*parent),
        Some(Parent(parent)) => containing_navnode(*parent, queries),
        None => None,
    }
}
/// All sibling focusables within a single NavNode
fn all_focusables(nav_node: Entity, queries: &NavQueries) -> Vec<Entity> {
    match queries.children.get(nav_node).ok() {
        Some(direct_children) => {
            let (mut focusables, others): (Vec<Entity>, Vec<Entity>) = direct_children
                .iter()
                .partition(|e| queries.focusables.get(**e).is_ok());
            let transitive_focusables = others
                .iter()
                .filter(|e| queries.nav_nodes.get(**e).is_err())
                .flat_map(|e| all_focusables(*e, queries));
            focusables.extend(transitive_focusables);
            focusables
        }
        None => vec![],
    }
}

fn get_active(
    focusables: &[Entity],
    actives: &Query<(), Or<(With<Active>, With<Focused>)>>,
) -> Option<Entity> {
    focusables
        .iter()
        .find(|focus| actives.get(**focus).is_ok())
        .cloned()
}

pub struct NavigationPlugin;
impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NavCommand>()
            .add_system(listen_nav_requests);
    }
}
