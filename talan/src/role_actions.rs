use linked_hash_set::LinkedHashSet;
use crate::macros::Action;
use log;
use xiv;

const ROLE_ACTIONS: [&str; 18] = [
    "careful synthesis ii",
    "careful synthesis",
    "comfort zone",
    "flawless synthesis",
    "hasty touch",
    "ingenuity ii",
    "ingenuity",
    "innovation",
    "maker's mark",
    "manipulation",
    "muscle memory",
    "piece by piece",
    "rapid synthesis",
    "reclaim",
    "rumination",
    "tricks of the trade",
    "waste not ii",
    "waste not",
];

#[derive(Debug)]
pub struct RoleActions<'a> {
    pub current_actions: LinkedHashSet<&'a str>,
}

// RoleActions is backed by a HashSet using a doubly linked list that can be used
// for LRU-like behavior, ensuring that as we add AdditionalActions they will be older
// actions not referenced in the current macro.
impl<'a> RoleActions<'a> {
    pub fn new() -> Self {
        RoleActions {
            current_actions: LinkedHashSet::new(),
        }
    }

    pub fn is_role_action(&self, action: &str) -> bool {
        ROLE_ACTIONS.contains(&action)
    }

    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.current_actions.len()
    }

    #[allow(dead_code)]
    pub fn contains(&self, action: &str) -> bool {
        self.current_actions.contains(action)
    }

    // Checks if |action| is in the list of role actions. If not, it
    // removes the oldest action from the list, adds the new action,
    // and returns the name of the action the crafting UI needs to remove
    // to make space.
    pub fn add_action(&mut self, action: &'a str) -> Option<Option<&str>> {
        if !self.is_role_action(action) {
            panic!("provided action is not a role action: `{}`", action);
        }

        // If the action exists in our list then nothing eeds to be done.
        if self.contains(action) {
            return None;
        }

        // If the action was added and fits within our limit of ten actions
        // then the caller should add it, but doesn't need to remove anything.
        if self.current_actions.insert(action) && self.current_actions.len() > 10 {
            return Some(None);
        }

        // Otherwise, we need to pop off the oldest action and let the caller know
        // to update the client's bookkeeping before adding the new action.

        return Some(Some(self.current_actions.pop_front().unwrap()));
    }
}

#[cfg(test)]
mod test {
    use super::RoleActions;
    use xiv;

    #[test]
    #[ignore]
    fn role_actions() {
        let mut ra = RoleActions::new(xiv::XivHandle{});
        ra.add_action("tricks of the trade");
        ra.add_action("reclaim");
        assert_eq!(2, ra.count());
        ra.add_action("ingenuity ii");
        ra.add_action("ingenuity");
        ra.add_action("innovation");
        ra.add_action("maker's mark");
        ra.add_action("manipulation");
        ra.add_action("muscle memory");
        ra.add_action("waste not ii");
        ra.add_action("hasty touch");
        assert_eq!(10, ra.count());
        ra.add_action("careful synthesis ii");
        assert_eq!(false, ra.contains("tricks of the trade"));
        assert_eq!(10, ra.count());
        ra.add_action("rumination");
        assert_eq!(false, ra.contains("reclaim"));
        assert_eq!(10, ra.count());
        println!("{:?}", ra);
    }
}
