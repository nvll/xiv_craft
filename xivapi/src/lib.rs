use failure::{format_err, Error};
use log;
use reqwest;
use serde::Deserialize;
use serde_json;
use std::cmp::Ordering;
use std::collections::HashMap;

const XIVAPI_SEARCH_URL: &str = "https://xivapi.com/search";

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct ItemIngredient {
    pub Name: String,
    pub ID: u32,
}

// These structures match the XIVApi schemas
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct CraftType {
    pub ID: u32,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RecipeLevelTable {
    pub ClassJobLevel: u32,
    pub Difficulty: u32,
    pub Durability: u32,
    pub ID: u32,
    pub Quality: u32,
    pub Stars: u32,
    pub SuggestedControl: u32,
    pub SuggestedCraftsmanship: u32,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct GameContentLinks {
    pub RecipeNotebookList: HashMap<String, Vec<u32>>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct ApiRecipe {
    pub ID: u32,
    pub Name: String,
    pub CraftType: CraftType,
    pub RecipeLevelTable: RecipeLevelTable,
    pub AmountIngredient0: u32,
    pub AmountIngredient1: u32,
    pub AmountIngredient2: u32,
    pub AmountIngredient3: u32,
    pub AmountIngredient4: u32,
    pub AmountIngredient5: u32,
    pub DifficultyFactor: u32,
    pub DurabilityFactor: u32,
    pub QualityFactor: u32,
    pub ItemIngredient0: Option<ItemIngredient>,
    pub ItemIngredient1: Option<ItemIngredient>,
    pub ItemIngredient2: Option<ItemIngredient>,
    pub ItemIngredient3: Option<ItemIngredient>,
    pub ItemIngredient4: Option<ItemIngredient>,
    pub ItemIngredient5: Option<ItemIngredient>,
    pub GameContentLinks: Option<GameContentLinks>,
}

impl ApiRecipe {
    // Through experimentation, the game appears to sort recipes based on
    // the following keys in priority order:
    //   1) Job ID (CRP < BSM < ARM < GSM < LTW < WVR < ALC < CUL)
    //   2) Craft Level
    //   3) Craft Stars
    //   4) RecipeNotebookList Row
    //   4) RecipeNotebookList Column
    fn key(&self) -> Result<(u32, u32, u32, u32, u32), Error> {
        let links = self
            .GameContentLinks
            .clone()
            .ok_or_else(|| format_err!("No GameContentLinks"))?;

        // RecipeNotebookList is organized into rows and columns. Below
        // represents column 9 in row 1053.
        //
        //  GameContentLinks {
        //     RecipeNotebookList: {
        //         "Recipe9": [
        //             1053,
        //         ],
        //     },
        // },

        let column_str = links.RecipeNotebookList.keys().collect::<Vec<_>>()[0];
        let column = column_str.trim_start_matches("Recipe").parse::<u32>()?;

        let row = links
            .RecipeNotebookList
            .get(column_str)
            .ok_or_else(|| (format_err!("Can't get column {}", column_str)))?[0];

        Ok((
            self.CraftType.ID,
            self.RecipeLevelTable.ClassJobLevel,
            self.RecipeLevelTable.Stars,
            row,
            column,
        ))
    }
}

impl PartialEq for ApiRecipe {
    fn eq(&self, other: &Self) -> bool {
        self.ID == other.ID
    }
}

impl Eq for ApiRecipe {}

impl Ord for ApiRecipe {
    fn cmp(&self, other: &Self) -> Ordering {
        if let Some(ord) = self.partial_cmp(other) {
            ord
        } else {
            // Fall back on comparing ID if partial_cmp fails.
            self.CraftType.ID.cmp(&other.CraftType.ID)
        }
    }
}

impl PartialOrd for ApiRecipe {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_key = self.key().ok()?;
        let other_key = other.key().ok()?;
        Some(self_key.cmp(&other_key))
    }
}

#[derive(Debug, Deserialize)]
struct ApiPagination {}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct ApiReply<T> {
    pub Pagination: ApiPagination,
    pub Results: Vec<T>,
}

pub fn query_recipe(item_name: &str) -> Result<Vec<ApiRecipe>, Error> {
    log::trace!("Looking up '{}'", item_name);
    let columns = [
        "AmountIngredient0",
        "AmountIngredient1",
        "AmountIngredient2",
        "AmountIngredient3",
        "AmountIngredient4",
        "AmountIngredient5",
        "CraftType.ID",
        "DifficultyFactor",
        "DurabilityFactor",
        "ID",
        "ItemIngredient0",
        "ItemIngredient1",
        "ItemIngredient2",
        "ItemIngredient3",
        "ItemIngredient4",
        "ItemIngredient5",
        "Name",
        "QualityFactor",
        "RecipeLevelTable",
        "GameContentLinks",
    ];
    let s: String = columns.iter().map(|e| e.to_string() + ",").collect();
    let body = reqwest::blocking::Client::new()
        .get(XIVAPI_SEARCH_URL)
        .query(&[
            ("indexes", "Recipe"),
            ("columns", &s),
            ("string", item_name.trim()),
            ("pretty", "1"),
        ])
        .send()?
        .text()?;
    let mut r: ApiReply<ApiRecipe> = serde_json::from_str(&body)?;
    r.Results.sort();
    log::trace!("{:#?}", r.Results);
    Ok(r.Results)
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

        let api_results = query_recipe("Rakshasa Axe")?;
        let item = &api_results[0];
        log::trace!("item fetched: {:#?}", item);
        assert_eq!(item.Name, "Rakshasa Axe");
        assert_eq!(item.CraftType.ID, 1);
        assert_eq!(item.AmountIngredient0, 3);
        assert_eq!(item.AmountIngredient1, 1);
        assert_eq!(item.AmountIngredient2, 1);
        assert_eq!(item.AmountIngredient3, 3);
        assert_eq!(item.AmountIngredient4, 0);
        assert_eq!(item.AmountIngredient5, 0);
        Ok(())
    }

    #[test]
    fn triphane_test() -> Result<(), Error> {
        setup();

        let api_results = query_recipe("Triphane")?;
        let item = &api_results[0];
        log::trace!("item fetched: {:#?}", item);
        assert_eq!(item.Name, "Triphane");
        Ok(())
    }

    #[test]
    fn swallowskin_gloves_test() -> Result<(), Error> {
        setup();

        let names = vec![
            "Swallowskin Gloves of Fending",
            "Swallowskin Gloves of Maiming",
            "Swallowskin Gloves of Striking",
            "Swallowskin Gloves of Scouting",
            "Swallowskin Gloves of Aiming",
            "Swallowskin Gloves of Casting",
            "Swallowskin Gloves of Healing",
            "Swallowskin Gloves",
        ];

        let api_results = query_recipe("Swallowskin Gloves")?;

        log::trace!("results: {:#?}", api_results);
        assert_eq!(api_results.len(), names.len());
        for (i, recipe) in api_results.iter().enumerate() {
            assert_eq!(recipe.Name, names[i]);
        }
        Ok(())
    }

    // TODO: After they added the Replica Sky Pirate gear it adjusted the list so Hemiskin
    // is now in a different spot in-game vs the API result. Need to figure out why, and if
    // it's a bug on the xivapi import end or ours.
    #[test]
    #[ignore]
    fn gloves_of_aiming_test() -> Result<(), Error> {
        setup();

        let names = vec![
            "Saurian Gloves of Aiming",
            "Archaeoskin Gloves of Aiming",
            "Dragonskin Gloves of Aiming",
            "Griffin Leather Gloves of Aiming",
            "Sky Pirate's Gloves of Aiming",
            "Replica High Allagan Gloves of Aiming",
            "Hemiskin Gloves of Aiming",
            "Sky Rat Fingerless Gloves of Aiming",
            "Gaganaskin Gloves of Aiming",
            "Replica Sky Pirate's Gloves of Aiming",
            "Replica Sky Rat Fingerless Gloves of Aiming",
            "Gyuki Leather Gloves of Aiming",
            "Tigerskin Gloves of Aiming",
            "Marid Leather Gloves of Aiming",
            "Slothskin Gloves of Aiming",
            "Gliderskin Gloves of Aiming",
            "Zonureskin Fingerless Gloves of Aiming",
            "Swallowskin Gloves of Aiming",
            "Brightlinen Long Gloves of Aiming",
            "Facet Halfgloves of Aiming",
        ];

        let api_results = query_recipe("gloves of aiming")?;

        log::trace!("results: {:#?}", api_results);
        assert_eq!(api_results.len(), names.len());
        for (i, recipe) in api_results.iter().enumerate() {
            assert_eq!(recipe.Name, names[i]);
        }
        Ok(())
    }
}
