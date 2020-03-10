use crate::action::{Action, ACTIONS};
use crate::recipe;
use anyhow::{anyhow, Result};
use imgui::ImString;
use serde::Deserialize;
use std::path::PathBuf;

// The |Toml| variant structures are used entirely for deserializing
// from a user friendly format into the actions necessary for Talan.
#[derive(Debug, Deserialize)]
pub struct MacroToml {
    pub name: String,
    pub durability: Vec<u32>,
    pub max_rlvl: Option<u32>,
    pub min_rlvl: Option<u32>,
    pub difficulty: Option<u32>,
    pub specialist: Option<bool>,
    pub actions: String,
}

#[derive(Debug, Deserialize)]
pub struct MacroFileToml {
    pub xiv_macro: Vec<MacroToml>,
}

#[derive(Clone, Debug)]
pub struct Macro {
    pub name: String,
    pub gui_name: ImString,
    pub durability: Vec<u32>,
    pub max_rlvl: Option<u32>,
    pub min_rlvl: Option<u32>,
    pub difficulty: Option<u32>,
    pub specialist: bool,
    pub actions: Vec<&'static Action>,
}

// Attempts to parse macros in |buffer| and return a list of actions.
fn parse_buffer(buffer: &str) -> Result<Vec<&'static Action>> {
    let mut actions = vec![];
    for line in buffer.trim().lines() {
        if line.trim().as_bytes()[0] == b'#' {
            log::trace!("skipping commented line: {}", line);
            continue;
        }
        let action = parse_line(line.trim())?;
        actions.push(action);
    }

    Ok(actions)
}

pub fn read_macros_from_buffer(buffer: &str, out_vec: &mut Vec<Macro>) -> Result<()> {
    let des = toml::from_str::<MacroFileToml>(buffer)?;
    for macro_toml in &des.xiv_macro {
        out_vec.push(Macro {
            name: macro_toml.name.clone(),
            gui_name: ImString::new(macro_toml.name.clone()),
            durability: macro_toml.durability.clone(),
            max_rlvl: macro_toml.max_rlvl,
            min_rlvl: macro_toml.min_rlvl,
            difficulty: macro_toml.difficulty,
            specialist: if let Some(spec) = macro_toml.specialist {
                spec
            } else {
                false
            },
            actions: parse_buffer(&macro_toml.actions)?,
        });
    }

    Ok(())
}

pub fn read_macros_from_file(path: &PathBuf, out_vec: &mut Vec<Macro>) -> Result<()> {
    read_macros_from_buffer(&std::fs::read_to_string(path)?, out_vec)
}

// Extract the action for a given line in a macro. Returns a
// String in the event of an error indicating a malformed macros.
pub fn parse_line(line: &str) -> Result<&'static Action> {
    let chars: Vec<char> = line.chars().collect();
    if chars.len() < 4 || chars[0] != '/' || chars[1] != 'a' || chars[2] != 'c' || chars[3] != ' ' {
        return Err(anyhow!("Macro is invalid: \"{}\"", line));
    }

    let mut has_quote = false;
    let mut action_string = String::new();
    let mut start = 4;
    let mut pos = start;
    while pos < chars.len() {
        let c = chars[pos];
        match c {
            c if c.is_alphanumeric() || c.is_whitespace() || c == '\'' => action_string.push(c),
            '\"' => {
                if !has_quote {
                    start += 1;
                    has_quote = true;
                } else {
                    break;
                }
            }
            '<' => break,
            _ => return Err(anyhow!("Error at character '{}'", chars[pos])),
        }
        pos += 1;
    }

    match ACTIONS.get(&*action_string.trim().to_lowercase()) {
        Some(action) => Ok(action),
        None => Err(anyhow!("Unknown action name \"{}\"", action_string)),
    }
}

// Determine the best macro available for a given recipe. If no match is found,
// return the first macro. Best match is determined by picking the macro who has
// the most matching parameters specified in the macro definition.
pub fn get_macro_for_recipe(macros: &[Macro], recipe: &recipe::Recipe, specialist: bool) -> usize {
    let mut best_pick = None;
    let mut best_match_cnt = 0;
    log::trace!("Selecting macro for \"{}\"", recipe.name);
    for (i, mcro) in macros.iter().enumerate() {
        let mut match_cnt = 0;
        if let Some(min_rlvl) = mcro.min_rlvl {
            if min_rlvl > recipe.level {
                log::trace!(
                    "\t[{}] macro minimum level {} > recipe level {}",
                    mcro.name,
                    min_rlvl,
                    recipe.level,
                );
                continue;
            }
            match_cnt += 1;
        }

        if let Some(max_rlvl) = mcro.max_rlvl {
            if max_rlvl < recipe.level {
                log::trace!(
                    "\t[{}] macro maximum level {} < recipe level {}",
                    mcro.name,
                    max_rlvl,
                    recipe.level,
                );
                continue;
            }
            match_cnt += 1;
        }

        // Match on difficulty if it exists
        if let Some(difficulty) = mcro.difficulty {
            if difficulty != recipe.difficulty {
                log::trace!("\t[{}] macro difficulty doesn't match", mcro.name,);
                continue;
            }
            match_cnt += 1;
        }

        if mcro.specialist {
            if mcro.specialist != specialist {
                log::trace!("\t[{}] macro speciality doesn't match", mcro.name,);
                continue;
            }
            match_cnt += 1;
        }

        if !mcro.durability.iter().any(|&d| d == recipe.durability) {
            log::trace!("\t[{}] macro durability doesn't match", mcro.name);
            continue;
        }
        match_cnt += 1;

        if match_cnt > best_match_cnt {
            best_match_cnt = match_cnt;
            best_pick = Some(i);
        }
        log::trace!("\t[{}] score = {}", mcro.name, match_cnt);
    }

    if best_pick.is_none() {
        log::error!("No suitable macro found for \"{}\"", recipe.name);
    }
    best_pick.unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{parse_line, MacroFileToml};
    use crate::action::ACTIONS;
    use crate::recipe::Recipe;

    const TEST_MACRO_TOML: &str = r#"
        [[xiv_macro]]
        name = "Test Macro"
        durability = [ 80 ]
        max_rlvl = 70
        actions = """
            /ac test string
            /ac string test <wait.2>
        """
    "#;

    #[test]
    fn toml_parse() {
        let m = toml::from_str::<MacroFileToml>(TEST_MACRO_TOML);
        println!("Parsed macro: {:#?}", m);
        assert!(m.is_ok());
    }

    #[test]
    fn multiple_durability() {
        const MACRO_BUFFER: &str = r#"
            [[xiv_macro]]
            name = "35 60"
            durability = [ 35, 60 ]
            actions = """"""

            [[xiv_macro]]
            name = "80"
            durability = [ 80 ]
            actions = """"""

            [[xiv_macro]]
            name = "40 70"
            durability = [ 40, 70 ]
            actions = """"""
        "#;

        let recipe_35 = Recipe {
            durability: 35,
            difficulty: 0,
            quality: 0,
            result_amount: 1,
            level: 0,
            specialist: false,
            id: 0,
            index: 0,
            job: 0,
            mats: Vec::new(),
            name: "35".to_string(),
        };
        let mut recipe_40 = recipe_35.clone();
        let mut recipe_60 = recipe_35.clone();
        let mut recipe_70 = recipe_35.clone();
        let mut recipe_80 = recipe_35.clone();
        recipe_40.durability = 40;
        recipe_60.durability = 60;
        recipe_70.durability = 70;
        recipe_80.durability = 80;
        let mut macros = Vec::new();
        assert!(super::read_macros_from_buffer(MACRO_BUFFER, &mut macros).is_ok());
        assert!(super::get_macro_for_recipe(&macros, &recipe_35, false) == 0);
        assert!(super::get_macro_for_recipe(&macros, &recipe_40, false) == 2);
        assert!(super::get_macro_for_recipe(&macros, &recipe_60, false) == 0);
        assert!(super::get_macro_for_recipe(&macros, &recipe_70, false) == 2);
        assert!(super::get_macro_for_recipe(&macros, &recipe_80, false) == 1);
    }

    #[test]
    fn macros_single_unqoted_no_wait() {
        // single word, unquoted, with no wait
        let entry = parse_line(r#"/ac Innovation"#).unwrap();
        assert_eq!(entry.name, "Innovation");
        assert_eq!(entry.wait_ms, 1500);
    }

    #[test]
    fn macros_single_qoted_no_wait() {
        // single word, quoted, with no wait
        let entry = parse_line(r#"/ac "Innovation""#).unwrap();
        assert_eq!(entry.name, "Innovation");
        assert_eq!(entry.wait_ms, 1500);
    }

    #[test]
    fn macros_single_unqoted_with_wait() {
        // single word, unquoted, with a wait
        let entry = parse_line(r#"/ac Innovation <wait.2>"#).unwrap();
        assert_eq!(entry.name, "Innovation");
        assert_eq!(entry.wait_ms, 1500);
    }

    #[test]
    fn macros_single_quoted_with_wait() {
        // single word, quoted, with a wait
        let entry = parse_line(r#"/ac "Innovation" <wait.2>"#).unwrap();
        assert_eq!(entry.name, "Innovation");
        assert_eq!(entry.wait_ms, 1500);
    }

    #[test]
    fn macros_double_quoted_no_wait() {
        // two words, quoted, with no wait
        let entry = parse_line(r#"/ac "Byregot's Blessing""#).unwrap();
        assert_eq!(entry.name, "Byregot's Blessing");
        assert_eq!(entry.wait_ms, 2500);
    }

    #[test]
    fn macros_double_quoted_with_wait() {
        // two words, quoted, with a wait
        let entry = parse_line(r#"/ac "Byregot's Blessing" <wait.3>"#).unwrap();
        assert_eq!(entry.name, "Byregot's Blessing");
        assert_eq!(entry.wait_ms, 2500);
    }

    #[test]
    fn macros_empty() {
        let result = parse_line(r#""#);
        assert_eq!(result.is_err(), true);
    }

    const TEST_MACRO_BUFFER: &str = r#"
        /ac "Inner Quiet" <wait.2>
        /ac "Great Strides" <wait.2>
        /ac "Manipulation" <wait.3>
        /ac "Byregot's Blessing" <wait.3>
        #/ac "Commented Action" <wait.1>
        /ac "Careful Synthesis" <wait.3>"#;

    #[test]
    fn macros_buffer() -> anyhow::Result<()> {
        let expected = [
            &ACTIONS.get("inner quiet").unwrap(),
            &ACTIONS.get("great strides").unwrap(),
            &ACTIONS.get("manipulation").unwrap(),
            &ACTIONS.get("byregot's blessing").unwrap(),
            &ACTIONS.get("careful synthesis").unwrap(),
        ];
        let actual = super::parse_buffer(TEST_MACRO_BUFFER)?;
        for (&left, right) in expected.iter().zip(actual.iter()) {
            assert_eq!(left, right);
        }
        Ok(())
    }
}
