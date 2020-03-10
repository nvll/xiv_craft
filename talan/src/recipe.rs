use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct RecipeMaterial {
    pub id: u32,
    pub count: u32,
    pub name: String,
}
// Top level structs to export out of the library
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Recipe {
    pub durability: u32,
    pub difficulty: u32,
    pub quality: u32,
    pub result_amount: u32,
    pub level: u32,
    pub specialist: bool,
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
            result_amount: item.AmountResult,
            id: item.ID,
            name: item.Name.clone(),
            specialist: (item.IsSpecializationRequired == 1),
            job: item.CraftType.ID as u32,
            index: 0,
            mats,
        }
    }
}

impl Recipe {
    // Searches the |results| slice passed back to us by Xivapi for a given item's recipe.
    // If |use_first| is set, then we will use the first item that matches the name, regardless
    // of the job that owns the recipe.
    pub fn filter(results: &[xivapi::ApiRecipe], name: &str, job: Option<u32>) -> Option<Recipe> {
        for (search_index, recipe) in results.iter().enumerate() {
            // Items like 'Cloud Pearl' also have 'Cloud Pearl Components' in
            // the results, and can have matches for multiple jobs. If there's
            // more than one job in the results then we should match on the one
            // requested. But if only a single result comes back it means the
            // user had the wrong job selected (For example, they searched
            // 'Dwarven Cotton Yarn' as a CRP. For ease of use in that
            // circumstance we'll just add it to the task list.
            if recipe.Name.to_lowercase() == name.to_lowercase()
                && (results.len() == 1
                    || job.is_none()
                    || job.unwrap() == recipe.CraftType.ID as u32)
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
    use super::Recipe;
    use anyhow::Result;
    use xivapi::query_recipe;

    #[test]
    fn bsm_cloud_pearl() -> Result<()> {
        let item_name = "Cloud Pearl";
        let query_results = query_recipe(item_name)?;
        assert!(!query_results.is_empty());
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
        let recipe = Recipe::filter(&query_results[..], item_name, Some(1));
        assert!(recipe.is_some());
        let r = recipe.unwrap();
        assert_eq!(r.name, item_name);
        assert_eq!(r.index, 3);
        Ok(())
    }

    // Verify that if we don't specify a job then we'll get the first result that matches the name.
    #[test]
    fn no_job_specified() -> Result<()> {
        // Cloud Pearl exists for all jobs, so the first result should be CRP.
        let tests = [("Cloud Pearl", 0), ("Tungsten Steel Ingot", 1)];
        for test in tests.iter() {
            let recipe = Recipe::filter(&query_recipe(test.0)?[..], test.0, None).unwrap();
            assert_eq!(recipe.name, test.0);
            assert_eq!(recipe.job, test.1);
        }
        Ok(())
    }

    #[test]
    fn specialization() -> Result<()> {
        let tests = [("Cloud Pearl", false), ("Ruby Barding", true)];
        for test in tests.iter() {
            let recipe = Recipe::filter(&query_recipe(test.0)?[..], test.0, None).unwrap();
            assert_eq!(recipe.name, test.0);
            assert_eq!(recipe.specialist, test.1);
        }
        Ok(())
    }
}
