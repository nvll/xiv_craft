use failure::Error;
use imgui::ImString;
use once_cell::sync::OnceCell;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct MacroFile {
    pub name: String,
    pub path: PathBuf,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Action {
    pub name: String,
    pub wait: u64,
}

// The |Toml| variant structures are used entirely for deserializing
// from a user friendly format into the actions necessary for Talan.
#[derive(Debug, Deserialize)]
pub struct MacroToml {
    pub name: String,
    pub durability: u32,
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

#[derive(Debug)]
pub struct Macro {
    pub name: String,
    pub gui_name: ImString,
    pub durability: u32,
    pub max_rlvl: Option<u32>,
    pub min_rlvl: Option<u32>,
    pub difficulty: Option<u32>,
    pub specialist: bool,
    pub actions: Vec<Action>,
}

static MACROS: OnceCell<Vec<Macro>> = OnceCell::new();

pub fn macros() -> &'static Vec<Macro> {
    MACROS.get().expect("Macros have not been initialized")
}

// A wrapper for |from_str| to read from the macros file.
pub fn from_path(path: &Path) -> Result<(), Error> {
    from_str(&std::fs::read_to_string(path)?)
}

pub fn from_str(s: &str) -> Result<(), Error> {
    let des = toml::from_str::<MacroFileToml>(s)?;
    let mut parsed_vec: Vec<Macro> = Vec::new();
    for macro_toml in &des.xiv_macro {
        parsed_vec.push(Macro {
            name: macro_toml.name.clone(),
            gui_name: ImString::new(macro_toml.name.clone()),
            durability: macro_toml.durability,
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

    MACROS.set(parsed_vec).expect("couldn't set up macro cache");
    Ok(())
}

lazy_static::lazy_static! {
    static ref ACTIONS: HashMap<&'static str, f64> = {
        let mut h = HashMap::new();
        h.insert("Advanced Synthesis", 2.5);
        h.insert("Advanced Touch", 2.5);
        h.insert("Basic Synthesis", 2.5);
        h.insert("Basic Touch", 2.5);
        h.insert("Brand of the Elements", 2.0);
        h.insert("Byregot's Blessing", 2.5);
        h.insert("Careful Observation", 2.0);
        h.insert("Delicate Synthesis", 2.5);
        h.insert("Final Appraisal", 2.0);
        h.insert("Great Strides", 2.0);
        h.insert("Hasty Touch", 2.5);
        h.insert("Ingenuity", 2.0);
        h.insert("Inner Quiet", 2.0);
        h.insert("Innovation", 2.0);
        h.insert("Intensive Synthesis", 2.5);
        h.insert("Master's Mend", 2.5);
        h.insert("Muscle Memory", 2.5);
        h.insert("Name of the Elements", 2.5);
        h.insert("Observe", 2.0);
        h.insert("Patient Touch", 2.5);
        h.insert("Precise Touch", 2.5);
        h.insert("Preparatory Touch", 2.5);
        h.insert("Prudent Touch", 2.5);
        h.insert("Rapid Synthesis", 2.5);
        h.insert("Reuse", 2.5);
        h.insert("Reflect", 2.5);
        h.insert("Standard Touch", 2.5);
        h.insert("Trained Eye", 2.5);
        h.insert("Tricks of the Trade", 2.5);
        h.insert("Waste Not", 2.5);
        h
    };
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Action ( action: {}, wait: {})", self.name, self.wait)
    }
}

// Attempts to parse macros in |buffer| and return a list of actions.
fn parse_buffer(buffer: &str) -> Result<Vec<Action>, Error> {
    let mut actions = vec![];
    for line in buffer.trim().lines() {
        if line.trim().as_bytes()[0] == b'#' {
            log::info!("skipping commented line: {}", line);
            continue;
        }
        actions.push(parse_line(line.trim())?);
    }

    Ok(actions)
}

// Extract the action and wait times for a given line in a macros. Returns a
// String in the event of an error indicating a malformed macros.
pub fn parse_line(line: &str) -> Result<Action, Error> {
    let re = Regex::new(r#"/ac ["]?([a-zA-Z:' ]+[a-zA-Z])["]?(?: <wait.([0-9])>)?"#)
        .expect("error compiling regex");
    let values = re
        .captures(line)
        .ok_or_else(|| failure::format_err!("Unable to parse line: `{}`", line))?;
    let action = values.get(1).map_or("", |m| m.as_str());
    let wait = match values.get(2) {
        Some(x) => x
            .as_str()
            .parse::<u64>()
            .map_err(|_| failure::format_err!("failed to parse as number: {}", x.as_str()))?,
        None => 3,
    };

    Ok(Action {
        name: action.to_lowercase(),
        wait,
    })
}

// Picks the most appropriate macro for a given set of recipe values. If none
// are found matching the durability then it will choose the last macro.
pub fn get_macro_for_recipe(
    durability: u32,
    rlvl: u32,
    difficulty: u32,
    specialist: bool,
) -> usize {
    for (i, m) in macros().iter().enumerate() {
        if let Some(m_min) = m.min_rlvl {
            if m_min > rlvl {
                continue;
            }
        }

        if let Some(m_max) = m.max_rlvl {
            if m_max < rlvl {
                continue;
            }
        }

        // Match on difficulty if it exists
        if let Some(m_difficulty) = m.difficulty {
            if m_difficulty != difficulty {
                continue;
            }
        }

        if m.durability == durability && m.specialist == specialist {
            return i;
        }
    }
    macros().len() - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MACRO_BUFFER: &str = r#"
        /ac "Comfort Zone" <wait.3>
        /ac "Inner Quiet" <wait.2>
        /ac "Great Strides" <wait.2>
        /ac "Manipulation II" <wait.3>
        /ac "Byregot's Blessing" <wait.3>
        #/ac "Commented Action" <wait.1>
        /ac "Careful Synthesis III" <wait.3>"#;

    const TEST_MACRO_TOML: &str = r#"
        [[xiv_macro]]
        name = "Test Macro"
        durability = 80
        max_rlvl = 390
        actions = """
            /ac test string
            /ac string test <wait.2>
        """"#;

    #[test]
    fn test_read() -> Result<(), Error> {
        let m = toml::from_str::<MacroFileToml>(TEST_MACRO_TOML);
        println!("output {:#?}", m);
        Ok(())
    }

    #[test]
    fn macros_single_unqoted_no_wait() {
        // single word, unquoted, with no wait
        let entry = parse_line(r#"/ac Innovation"#).unwrap();
        assert_eq!(entry.name, "innovation");
        assert_eq!(entry.wait, 3);
    }

    #[test]
    fn macros_specialty() {
        // single word, unquoted, with no wait
        let entry = parse_line(r#"/ac "Specialty: Reflect""#).unwrap();
        assert_eq!(entry.name, "specialty: reflect");
        assert_eq!(entry.wait, 3);
    }

    #[test]
    fn macros_single_qoted_no_wait() {
        // single word, quoted, with no wait
        let entry = parse_line(r#"/ac "Innovation""#).unwrap();
        assert_eq!(entry.name, "innovation");
        assert_eq!(entry.wait, 3);
    }

    #[test]
    fn macros_single_unqoted_with_wait() {
        // single word, unquoted, with a wait
        let entry = parse_line(r#"/ac Innovation <wait.2>"#).unwrap();
        assert_eq!(entry.name, "innovation");
        assert_eq!(entry.wait, 2);
    }

    #[test]
    fn macros_single_quoted_with_wait() {
        // single word, quoted, with a wait
        let entry = parse_line(r#"/ac "Innovation" <wait.2>"#).unwrap();
        assert_eq!(entry.name, "innovation");
        assert_eq!(entry.wait, 2);
    }

    #[test]
    fn macros_double_quoted_no_wait() {
        // two words, quoted, with no wait
        let entry = parse_line(r#"/ac "Byregot's Blessing""#).unwrap();
        assert_eq!(entry.name, "byregot's blessing");
        assert_eq!(entry.wait, 3);
    }

    #[test]
    fn macros_double_quoted_with_wait() {
        // two words, quoted, with a wait
        let entry = parse_line(r#"/ac "Byregot's Blessing" <wait.3>"#).unwrap();
        assert_eq!(entry.name, "byregot's blessing");
        assert_eq!(entry.wait, 3);
    }

    #[test]
    fn macros_empty() {
        let result = parse_line(r#""#);
        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn macros_buffer() -> Result<(), Error> {
        let actual = parse_buffer(TEST_MACRO_BUFFER)?;
        assert_eq!(validate_test_entries(actual), true);
        Ok(())
    }

    fn validate_test_entries(actual: Vec<Action>) -> bool {
        let expected = [
            Action {
                name: "comfort zone".to_string(),
                wait: 3,
            },
            Action {
                name: "inner quiet".to_string(),
                wait: 2,
            },
            Action {
                name: "great strides".to_string(),
                wait: 2,
            },
            Action {
                name: "manipulation ii".to_string(),
                wait: 3,
            },
            Action {
                name: "byregot's blessing".to_string(),
                wait: 3,
            },
            Action {
                name: "careful synthesis iii".to_string(),
                wait: 3,
            },
        ];

        (actual == expected)
    }
}
