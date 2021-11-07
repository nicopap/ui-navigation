//! Construct and navigate the navigation tree
mod events;

use events::{Command, Event};

// The tree structure is implicit to the order of the NavigationNodes:
// The root is the first element of the Vec, since a Container has at most a
// single child Container, we just implicitly declare its the following node
// in the Vec. The node is terminal when there is no following node in the Vec
// (ie: it's the last one of the Vec)
/// The hierarchical relation of focusable elements.
///
/// The tree most at least have one branch, and each branch must at least
/// have one focusable.
///
/// Those invariables are enforced at the API level.
///
/// `Focusables` within a signle branch can be navigated through freely. Either
/// with `NavCommand::Move*` or `NavCommand::{Previous, Next}` depending on
/// {TODO: TBD}. Using `NavCommand::{Action, Cancel}`, you can go down and up
/// the branches.
///
/// The `Active` elements are `branch.focusables[branch.active]`. By
/// construction, the `Focused` element is
/// `tree.branches.last().focusables[tree.branches.last().active]`.
///
/// TODO: when adding Cancel/Action navigation, it will be
/// `tree.branches[tree.focused]` instead of `.last()`.
pub struct Tree<T> {
    /// Monotonically increasing counter
    last_branch_version: BranchVersion,
    branches: Vec<Branch<T>>,
}
struct Branch<T> {
    version: BranchVersion,
    active: usize,
    nav_node: T,
    focusables: Vec<T>,
}
impl<T> Branch<T> {
    fn new(old_version: Option<BranchVersion>, focusable: T, nav_node: T) -> Self {
        Branch {
            version: BranchVersion(old_version.map_or(0, |old| old.0 + 1)),
            active: 0,
            focusables: vec![focusable],
            nav_node,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
struct BranchVersion(usize);

#[derive(Copy, Clone, PartialEq)]
pub struct BranchId {
    index: usize,
    version: BranchVersion,
}
impl BranchId {
    fn new(index: usize, version: BranchVersion) -> BranchId {
        BranchId { index, version }
    }
}

impl<T> Tree<T> {
    pub fn new(focusable: T, root_node: T) -> Self {
        let mut tree = Tree {
            last_branch_version: BranchVersion(0),
            branches: Vec::new(),
        };
        tree.add_branch(focusable, root_node);
        tree
    }
    pub fn add_navigable(&mut self, branch_id: BranchId, focusable: T) -> Option<()> {
        let branch = self.branches.get_mut(branch_id.index)?;
        if branch.version != branch_id.version {
            return None;
        }
        branch.focusables.push(focusable);
        Some(())
    }
    pub fn add_branch(&mut self, focusable: T, nav_node: T) -> BranchId {
        let new_branch = Branch::new(Some(self.last_branch_version), focusable, nav_node);
        let last_branch_version = new_branch.version;
        self.branches.push(new_branch);
        self.last_branch_version = last_branch_version;
        BranchId::new(self.branches.len() - 1, last_branch_version)
    }
    fn replace_branches(
        &mut self,
        extending: BranchId,
        focusable: T,
        nav_node: T,
    ) -> Option<BranchId> {
        let branch = self.branches.get(extending.index)?;
        if branch.version != extending.version {
            return None;
        }

        self.branches.truncate(extending.index + 1);
        Some(self.add_branch(focusable, nav_node))
    }
    pub fn branch_of(&self, node: T) -> Option<BranchId>
    where
        T: PartialEq,
    {
        let (index, branch) = self
            .branches
            .iter()
            .enumerate()
            .find(|branch| branch.1.nav_node == node)?;
        Some(BranchId::new(index, branch.version))
    }

    fn active_trail(&self, up_to_branch: usize) -> impl Iterator<Item = &T> {
        self.branches
            .iter()
            .skip(up_to_branch)
            .map(|branch| &branch.focusables[branch.active])
    }

    fn focused(&self) -> T
    where
        T: Copy,
    {
        let last_branch = self.branches.last().unwrap();
        last_branch.focusables[last_branch.active]
    }

    fn change_focus_at(&self, command: Command, current_branch: usize) -> Event<T>
    where
        T: Located + Copy,
    {
        let focused_branch = &self.branches[current_branch];
        let focusables = &focused_branch.focusables;
        let focused = focusables[focused_branch.active];
        let direction = match Direction::try_from(command) {
            Ok(direction) => direction,
            Err(_) => {
                return Event::Caught {
                    container: focused_branch.nav_node,
                    command,
                    focused: self.focused(),
                }
            }
        };
        let next_focused = focused.closest_in_direction(direction, focusables);
        match next_focused {
            Some(to) => {
                let disactivated = self.active_trail(current_branch).cloned().collect();
                Event::FocusChanged { to, disactivated }
            }
            None if current_branch == 0 => {
                let focused = self.focused();
                Event::Uncaught { command, focused }
            }
            None => self.change_focus_at(command, current_branch - 1),
        }
    }
    pub fn change_focus(&self, command: Command) -> Event<T>
    where
        T: Located + Copy,
    {
        let last_branch = self.branches.len() - 1;
        self.change_focus_at(command, last_branch)
    }
}

// Alternative design: Instead of evaluating at focus-change time the
// neighbores, somehow cache them
// (actually, `trait Located` may already enable that)
pub enum Direction {
    South,
    North,
    East,
    West,
}
impl TryFrom<Command> for Direction {
    type Error = ();
    fn try_from(value: Command) -> Result<Self, Self::Error> {
        use Command::*;
        use Direction::*;
        match value {
            MoveUp => Ok(North),
            MoveDown => Ok(South),
            MoveLeft => Ok(West),
            MoveRight => Ok(East),
            _ => Err(()),
        }
    }
}

pub trait Located: Sized {
    fn closest_in_direction(&self, direction: Direction, others: &[Self]) -> Option<Self>;
}
