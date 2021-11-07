mod events;

use std::cmp::Ordering;

use bevy::ecs::system::QuerySingleError;
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;

pub use crate::events::{Command as NavCommand, NavEvent};

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
    direction: Direction,
    focused: Entity,
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
    command: NavCommand,
    focused: Entity,
    siblings: &[Entity],
    transform: &Query<&GlobalTransform>,
) -> Option<Entity> {
    let direction: Direction = command.try_into().ok()?;
    move_focus_at(direction, focused, siblings, transform)
}

fn rootless_change_focus(
    command: NavCommand,
    focused: Entity,
    is_focusable: &Query<Entity, With<Focusable>>,
    transform: &Query<&GlobalTransform>,
    mut disactivated: Vec<Entity>,
) -> NavEvent {
    let siblings: Vec<Entity> = is_focusable.iter().collect();
    disactivated.push(focused);
    match change_focus_at(command, focused, &siblings, transform) {
        Some(to) => NavEvent::FocusChanged { to, disactivated },
        None => NavEvent::Uncaught { command, focused },
    }
}

fn change_focus(
    focused: Entity,
    command: NavCommand,
    children: &Query<&Children>,
    parents: &Query<&Parent>,
    is_focusable: &Query<Entity, With<Focusable>>,
    is_nav_node: &Query<(), With<NavNode>>,
    is_active: &Query<(), Or<(With<Active>, With<Focused>)>>,
    transform: &Query<&GlobalTransform>,
    mut disactivated: Vec<Entity>,
) -> NavEvent {
    let nav_node = match containing_navnode(focused, parents, is_nav_node) {
        Some(entity) => entity,
        None => {
            return rootless_change_focus(command, focused, is_focusable, transform, disactivated)
        }
    };
    let siblings = all_focusables(nav_node, children, is_focusable, is_nav_node);
    // TODO: better handling of missing Active/Focused
    let focused = get_active(&siblings, is_active).unwrap_or(*siblings.first().unwrap());
    disactivated.push(focused);
    match change_focus_at(command, focused, &siblings, transform) {
        Some(to) => NavEvent::FocusChanged { to, disactivated },
        None => change_focus(
            nav_node,
            command,
            children,
            parents,
            is_focusable,
            is_nav_node,
            is_active,
            transform,
            disactivated,
        ),
    }
}
// Consideration: exploring a part of the UI graph every time a navigation
// request is sent might be too slow. Possible mitigation: caching parts of the
// navigation graph.
fn listen_nav_requests(
    mut commands: Commands,
    mut events: EventReader<NavCommand>,
    children: Query<&Children>,
    parents: Query<&Parent>,
    is_focusable: Query<Entity, With<Focusable>>,
    is_nav_node: Query<(), With<NavNode>>,
    is_active: Query<(), Or<(With<Active>, With<Focused>)>>,
    focused: Query<Entity, With<Focused>>,
    // TODO: may be wise to abstract this away
    transform: Query<&GlobalTransform>,
) {
    // TODO: this most likely breaks when there is more than a single event
    for command in events.iter() {
        let focused_id = focused.get_single().unwrap_or_else(|err| {
            if matches!(err, QuerySingleError::MultipleEntities(_)) {
                panic!("Multiple focused, not possible");
            }
            is_focusable.iter().next().unwrap()
        });
        let change = change_focus(
            focused_id,
            *command,
            &children,
            &parents,
            &is_focusable,
            &is_nav_node,
            &is_active,
            &transform,
            Vec::new(),
        );
        if let NavEvent::FocusChanged { to, disactivated } = change {
            for elem in disactivated {
                commands.entity(elem).remove::<Focused>();
                commands.entity(elem).remove::<Active>();
            }
            commands.entity(to).insert(Focused);
        }
    }
}

fn containing_navnode(
    focusable: Entity,
    parents: &Query<&Parent>,
    is_nav_node: &Query<(), With<NavNode>>,
) -> Option<Entity> {
    match parents.get(focusable).ok() {
        Some(Parent(parent)) if is_nav_node.get(*parent).is_ok() => Some(*parent),
        Some(Parent(parent)) => containing_navnode(*parent, parents, is_nav_node),
        None => None,
    }
}
/// All sibling focusables within a single NavNode
fn all_focusables(
    nav_node: Entity,
    children: &Query<&Children>,
    is_focusable: &Query<Entity, With<Focusable>>,
    is_nav_node: &Query<(), With<NavNode>>,
) -> Vec<Entity> {
    match children.get(nav_node).ok() {
        Some(direct_children) => {
            let (mut focusables, others): (Vec<Entity>, Vec<Entity>) = direct_children
                .iter()
                .partition(|e| is_focusable.get(**e).is_ok());
            let transitive_focusables = others
                .iter()
                .filter(|e| is_nav_node.get(**e).is_err())
                .flat_map(|e| all_focusables(*e, children, is_focusable, is_nav_node));
            focusables.extend(transitive_focusables);
            focusables
        }
        None => vec![],
    }
}

fn get_active(
    focusables: &[Entity],
    is_active: &Query<(), Or<(With<Active>, With<Focused>)>>,
) -> Option<Entity> {
    focusables
        .iter()
        .find(|focus| is_active.get(**focus).is_ok())
        .cloned()
}

pub struct NavigationPlugin;
impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NavCommand>()
            .add_system(listen_nav_requests);
    }
}
