use serde::{Deserialize, Serialize};
use xivapi;

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct MaterialCount {
    pub nq: u32,
    pub hq: u32,
}

// A task represents crafting a specific item a given number of times
// using a provided recipe and macro. mat_quality is a specific field
// separate from Recipe because the Recipe type is from an external
// crate.
#[derive(PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Task {
    pub ignore_mat_quality: bool,
    pub is_collectable: bool, // craft collectables
    pub macro_id: i32,
    pub mat_quality: Vec<MaterialCount>,
    pub quantity: i32, // number of items to craft
    pub recipe: xivapi::Recipe,
}
