use failure::Error;
use log;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;

const XIVAPI_SEARCH_URL: &str = "https://xivapi.com/search";

#[derive(PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Material {
    pub id: i32,
    pub count: i32,
    pub name: String,
}

// Top level structs to export out of the library
#[derive(PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Recipe {
    pub id: i32,
    pub name: String,
    pub can_hq: bool,
    pub job: u32,
    pub index: usize,
    pub mats: Vec<Material>,
}

fn convert_ingredient(r: &mut Recipe, ii: &Option<ItemIngredient>, amount: i32) {
    if let Some(ref ii) = ii {
        r.mats.push(Material {
            id: ii.ID,
            name: ii.Name.clone(),
            count: amount,
        });
    }
}

// Rather than From, we probably need a method to find the right recipe and fill in the offset
impl Recipe {
    fn from_results(recipes: Vec<ApiRecipe>, item_name: &str, job: u32) -> Option<Recipe> {
        // Figure
        for (i, recipe) in recipes.iter().enumerate() {
            // Items like 'Cloud Pearl' also have 'Cloud Pearl Components' in the results,
            // and can have multiple jobs. If there's more than one job in the results then
            // we should match on the one requested. But if only result comes back it means
            // the user had the wrong job selected. For ease of use in that circumstance
            // we'll just add it to the task list.
            if recipe.Name.to_lowercase() == item_name.to_lowercase()
                && (recipes.len() == 1 || recipe.CraftType.ID as u32 == job)
            {
                let mut r = Recipe {
                    id: recipe.ID,
                    name: recipe.Name.clone(),
                    can_hq: recipe.CanHq != 0,
                    job: recipe.CraftType.ID as u32,
                    index: i,
                    mats: Vec::new(),
                };
                convert_ingredient(&mut r, &recipe.ItemIngredient0, recipe.AmountIngredient0);
                convert_ingredient(&mut r, &recipe.ItemIngredient1, recipe.AmountIngredient1);
                convert_ingredient(&mut r, &recipe.ItemIngredient2, recipe.AmountIngredient2);
                convert_ingredient(&mut r, &recipe.ItemIngredient3, recipe.AmountIngredient3);
                convert_ingredient(&mut r, &recipe.ItemIngredient4, recipe.AmountIngredient4);
                convert_ingredient(&mut r, &recipe.ItemIngredient5, recipe.AmountIngredient5);
                return Some(r);
            }
        }
        None
    }
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct ItemIngredient {
    Name: String,
    ID: i32,
}

// These structures match the XIVApi schemas
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct CraftType {
    ID: i32,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct ApiRecipe {
    ID: i32,
    Name: String,
    CanHq: i32,
    CraftType: CraftType,
    AmountIngredient0: i32,
    AmountIngredient1: i32,
    AmountIngredient2: i32,
    AmountIngredient3: i32,
    AmountIngredient4: i32,
    AmountIngredient5: i32,
    ItemIngredient0: Option<ItemIngredient>,
    ItemIngredient1: Option<ItemIngredient>,
    ItemIngredient2: Option<ItemIngredient>,
    ItemIngredient3: Option<ItemIngredient>,
    ItemIngredient4: Option<ItemIngredient>,
    ItemIngredient5: Option<ItemIngredient>,
}

#[derive(Debug, Deserialize)]
struct ApiPagination {}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct ApiReply<T> {
    pub Pagination: ApiPagination,
    pub Results: Vec<T>,
}

pub fn query_recipe_by_name(item_name: &str) -> Result<Vec<ApiRecipe>, Error> {
    log::trace!("Looking up '{}'", item_name);
    let mut columns = String::new() + "ID,Name,CanHq,CraftType.ID";
    for i in 0..=5 {
        columns += format!(",AmountIngredient{},ItemIngredient{}", i, i).as_str();
    }
    let body = reqwest::Client::new()
        .get(XIVAPI_SEARCH_URL)
        .query(&[
            ("indexes", "Recipe"),
            ("columns", columns.as_str()),
            ("string", item_name),
            ("sort_field", "ID"), // XIV sorts recipe output in game by ID of item in the recipe list
            ("pretty", "1"),
        ])
        .send()?
        .text()?;
    let r: ApiReply<ApiRecipe> = serde_json::from_str(&body)?;
    log::trace!("{:#?}", r.Results);
    Ok(r.Results)
}

pub fn get_recipe_for_job(item_name: &str, job: u32) -> Result<Option<Recipe>, Error> {
    Ok(Recipe::from_results(
        query_recipe_by_name(item_name)?,
        item_name,
        job,
    ))
}

#[cfg(test)]
mod test {
    use std::sync::Once;
    static START: Once = Once::new();

    fn setup() {
        START.call_once(|| {
            env_logger::init();
        });
    }

    use super::*;
    #[test]
    fn basic_get_test() -> Result<(), Error> {
        setup();

        let api_results = query_recipe_by_name("Rakshasa Axe")?;
        let item = &api_results[0];
        log::trace!("item fetched: {:#?}", item);
        assert_eq!(item.Name, "Rakshasa Axe");
        assert_eq!(item.CraftType.ID, 1);
        assert_eq!(item.CanHq, 1);
        assert_eq!(item.AmountIngredient0, 3);
        assert_eq!(item.AmountIngredient1, 1);
        assert_eq!(item.AmountIngredient2, 1);
        assert_eq!(item.AmountIngredient3, 3);
        assert_eq!(item.AmountIngredient4, 0);
        assert_eq!(item.AmountIngredient5, 0);
        Ok(())
    }
}
