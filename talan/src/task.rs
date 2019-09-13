use crate::recipe::Recipe;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct MaterialCount {
    pub nq: u32,
    pub hq: u32,
}

// A task represents crafting a specific item a given number of times
// using a provided recipe and macro.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Task {
    pub use_any_mats: bool,
    pub is_collectable: bool, // craft collectables
    pub macro_id: i32,
    pub mat_quality: Vec<MaterialCount>,
    pub quantity: u32, // number of items to craft
    pub recipe: Recipe,
}

impl Task {
    pub fn new(recipe: Recipe, count: u32) -> Task {
        Task {
            use_any_mats: true,
            is_collectable: false,
            macro_id: 0,
            quantity: count,
            mat_quality: recipe
                .mats
                .iter()
                .map(|m| MaterialCount { nq: m.count, hq: 0 })
                .collect(),
            recipe,
        }
    }
}

// Used to represent the status of a Task being executed by the crafting
// engine.
#[derive(Clone, Debug)]
pub struct Status {
    pub name: String,
    pub finished: u32,
    pub total: u32,
}

impl<'a> From<&'a Task> for Status {
    fn from(task: &'a Task) -> Self {
        Status {
            name: task.recipe.name.clone(),
            finished: 0,
            total: task.quantity as u32,
        }
    }
}
