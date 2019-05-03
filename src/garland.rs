use failure::Error;
use log;
use reqwest;
use serde_json;
use std::fmt;
use url::form_urlencoded;

impl fmt::Display for JsonItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "item {{")?;
        writeln!(f, "\tname: {}", self.item.name)?;
        write!(f, "\tid:   {}", self.item.id)?;
        write!(f, "\tingredients; {{")?;
        for (i, elem) in self.ingredients.iter().enumerate() {
            writeln!(
                f,
                "\t\t {}x {} (id: {})",
                self.item.craft[0].ingredients[i].amount, elem.name, elem.id
            )?;
        }
        writeln!(f, "\t}}")?;
        writeln!(f, "}}")
    }
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug)]
struct JsonItemSearchResult {
    id: String,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug)]
struct JsonItem {
    item: JsonItemData,
    ingredients: Vec<JsonItemIngredient>,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug)]
struct JsonItemData {
    name: String,
    id: u64,
    craft: Vec<JsonCraft>,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug)]
struct JsonItemIngredient {
    id: u64,
    name: String,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug)]
struct JsonCraft {
    job: u64,
    quality: u64,
    progress: u64,
    ingredients: Vec<JsonCraftIngredient>,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize, Debug)]
struct JsonCraftIngredient {
    id: u64,
    amount: u64,
    quality: Option<u64>,
}

#[derive(Debug)]
pub struct Item {
    pub name: String,
    pub materials: Vec<Material>,
}

#[derive(Debug)]
pub struct Material {
    pub id: u64,
    pub name: String,
    pub count: u64,
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.name)?;
        writeln!(f, "[")?;
        for m in &self.materials {
            writeln!(f, "  {}x {}", m.count, m.name)?;
        }
        writeln!(f, "]")
    }
}

// Convert Garland's json layout to a structure easier to use
// for Talan's purposes.
impl From<JsonItem> for Item {
    fn from(json_item: JsonItem) -> Self {
        // The JSON layout keeps terse info like id/amount in the
        // craft ingredients, but keeps all the information about
        // each of those in the top level. The data is all extracted
        // and combined in this conversion method.
        let mut v = Vec::new();
        for craft_item in &json_item.item.craft[0].ingredients {
            // Ignore shards, crystals, and clusters
            if craft_item.id <= 19 {
                continue;
            }

            let mut name = String::new();
            for ingredient in &json_item.ingredients {
                if craft_item.id == ingredient.id {
                    name = ingredient.name.clone();
                }
            }

            v.push(Material {
                id: craft_item.id,
                count: craft_item.amount,
                name: name.to_string(),
            });
        }

        Item {
            name: json_item.item.name,
            materials: v,
        }
    }
}

// Return the item id for the provided item name
pub fn query_item_id(item_name: &str) -> Result<Option<u64>, Error> {
    let garland_search_url = String::from("https://www.garlandtools.org/api/search.php?");
    let encoded_url: String = form_urlencoded::Serializer::new(garland_search_url)
        .append_pair("craftable", "1")
        .append_pair("type", "item")
        .append_pair("text", item_name)
        .append_pair("lang", "en")
        .append_pair("exact", "1")
        .finish();
    log::trace!("fetch({})", encoded_url);
    let body = reqwest::get(&encoded_url)?.text()?;
    let items: Vec<JsonItemSearchResult> = serde_json::from_str(&body)?;
    // We should not get duplicates, but use just the first if we do
    println!("items: {:?}", items);
    if items.is_empty() {
        return Err(failure::format_err!("item `{}` not found", item_name));
    }
    let id: u64 = items[0].id.parse()?;
    Ok(Some(id))
}

// Get the materials and other information for a given item
pub fn fetch_item_info(name: &str) -> Result<Item, Error> {
    let id = query_item_id(&name)?.unwrap();
    let garland_item_url = String::from("http://www.garlandtools.org/db/doc/item/en/3/");
    let encoded_url = format!("{}{}.json", garland_item_url, id);
    log::trace!("fetch({})", encoded_url);
    let body = reqwest::get(&encoded_url)?.text()?;
    let item: JsonItem = serde_json::from_str(&body)?;

    Ok(Item::from(item))
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn query_rakshasa_dogi_of_casting() {
        const RAKSHASA_DOGI_OF_CASTING_ID: u64 = 23821;
        let id = query_item_id(&"Rakshasa Dogi of Casting").unwrap().unwrap();
        assert_eq!(id, RAKSHASA_DOGI_OF_CASTING_ID);
    }

    #[test]
    fn query_crimson_cider_recipe() {
        let item = fetch_item_info("Crimson Cider").unwrap();
        assert_eq!(item.name, "Crimson Cider");
        assert_eq!(item.materials[0].name, "Crimson Pepper");
        assert_eq!(item.materials[0].count, 1);
        assert_eq!(item.materials[1].name, "Jhammel Ginger");
        assert_eq!(item.materials[1].count, 1);
        assert_eq!(item.materials[2].name, "Cumin Seeds");
        assert_eq!(item.materials[2].count, 1);
        assert_eq!(item.materials[3].name, "Kudzu Root");
        assert_eq!(item.materials[3].count, 1);
        assert_eq!(item.materials[4].name, "Loquat");
        assert_eq!(item.materials[4].count, 3);
    }

    // This test verifies whether we receive the right information
    // for items that have multiple recipes associated with them, or
    // names that happen to be a substring of another item that is also
    // craftable. The common case for this are Custom Delivery items where
    // there is often a 'Compoement' item that can be crafted for lower level
    // folks, as well as the item itself.
    //
    // This tests for 'Sui-no-Sato Special' to ensure we don't get confused by
    // 'Sui-no-Sato Special Component'
    #[test]
    fn query_turnin_item() {
        let item = fetch_item_info("Sui-no-Sato Special").unwrap();
        println!("{}", item);
        assert_eq!(item.name, "Sui-no-Sato Special");
        assert_eq!(item.materials.len(), 1);
        assert_eq!(item.materials[0].name, "Sui-no-Sato Special Components");
        assert_eq!(item.materials[0].count, 3);
    }
}
