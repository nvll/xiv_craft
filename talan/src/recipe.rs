use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct RecipeMaterial {
    pub id: u32,
    pub count: u32,
    pub name: String,
}
// Top level structs to export out of the library
#[derive(PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Recipe {
    pub durability: u32,
    pub difficulty: u32,
    pub quality: u32,
    pub level: u32,
    pub id: u32,
    pub index: usize,
    pub job: u32,
    pub mats: Vec<RecipeMaterial>,
    pub name: String,
}

impl From<&xivapi::ApiRecipe> for Recipe {
    fn from(item: &xivapi::ApiRecipe) -> Self {
        let recipe_mats = [
            (&item.ItemIngredient0, item.AmountIngredient0),
            (&item.ItemIngredient1, item.AmountIngredient1),
            (&item.ItemIngredient2, item.AmountIngredient2),
            (&item.ItemIngredient3, item.AmountIngredient3),
            (&item.ItemIngredient4, item.AmountIngredient4),
            (&item.ItemIngredient5, item.AmountIngredient5),
        ];

        let mut mats = Vec::new();
        for (opt_mat, cnt) in recipe_mats.iter() {
            if let Some(mat) = opt_mat {
                mats.push(RecipeMaterial {
                    id: mat.ID,
                    name: mat.Name.to_owned(),
                    count: *cnt,
                });
            }
        }

        Recipe {
            level: item.RecipeLevelTable.ClassJobLevel,
            durability: (item.RecipeLevelTable.Durability * item.DurabilityFactor) / 100,
            difficulty: (item.RecipeLevelTable.Difficulty * item.DifficultyFactor) / 100,
            quality: (item.RecipeLevelTable.Quality * item.QualityFactor) / 100,
            id: item.ID,
            name: item.Name.clone(),
            job: item.CraftType.ID as u32,
            index: 0,
            mats,
        }
    }
}

pub struct RecipeBuilder<'a> {
    name: &'a str,
    job: u32,
}

impl<'a> RecipeBuilder<'a> {
    pub fn new(name: &'a str, job: u32) -> Self {
        RecipeBuilder { name, job }
    }

    pub fn from_results(self, results: &[xivapi::ApiRecipe]) -> Option<Recipe> {
        for (search_index, recipe) in results.iter().enumerate() {
            // Items like 'Cloud Pearl' also have 'Cloud Pearl Components' in
            // the results, and can have matches for multiple jobs. If there's
            // more than one job in the results then we should match on the one
            // requested. But if only a single result comes back it means the
            // user had the wrong job selected (For example, they searched
            // 'Dwarven Cotton Yarn' as a CRP. For ease of use in that
            // circumstance we'll just add it to the task list.
            if recipe.Name.to_lowercase() == self.name.to_lowercase()
                && (results.len() == 1 || recipe.CraftType.ID as u32 == self.job)
            {
                let mut r: Recipe = Recipe::from(recipe);
                r.index = search_index;
                return Some(r);
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use failure::Error;

    #[test]
    fn bsm_cloud_pearl_test() -> Result<(), Error> {
        let item_name = "Cloud Pearl";
        let query_results = xivapi::query_recipe(item_name)?;
        assert!(query_results.len() > 0);
        // 1 = BSM. The results for Cloud Pearl will look like:
        // CRP ---
        // Cloud Pearl
        // Cloud Pearl Components
        // BSM ---
        // Cloud Pearl
        // Cloud Pearl Components
        //
        // This test ensures that we recieve index 3 for a specific search and don't
        // get an index of a 'Components' item.
        let recipe = RecipeBuilder::new(item_name, 1).from_results(&query_results[..]);
        assert!(recipe.is_some());
        let r = recipe.unwrap();
        assert_eq!(r.name, item_name);
        assert_eq!(r.index, 3);
        Ok(())
    }
}
