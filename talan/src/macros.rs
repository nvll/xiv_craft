use anyhow::anyhow;
use imgui::ImString;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct MacroFile {
    pub name: String,
    pub path: PathBuf,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Action {
    pub name: &'static str,
    pub wait_ms: u64,
}

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

#[derive(Debug)]
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

static MACROS: OnceCell<Vec<Macro>> = OnceCell::new();

pub fn macros() -> &'static Vec<Macro> {
    MACROS.get().expect("Macros have not been initialized")
}

pub fn from_path(path: &Path) -> anyhow::Result<()> {
    from_str(&std::fs::read_to_string(path)?)
}

lazy_static::lazy_static! {
    static ref ACTIONS: HashMap<&'static str, Action> = {
        let mut h = HashMap::new();
        // Buff actions
        h.insert("final appraisal", Action { name: "Final Appraisal",  wait_ms: 1500 });
        h.insert("great strides", Action { name: "Great Strides",  wait_ms: 1500 });
        h.insert("ingenuity", Action { name: "Ingenuity",  wait_ms: 1500 });
        h.insert("inner quiet", Action { name: "Inner Quiet",  wait_ms: 1500 });
        h.insert("innovation", Action { name: "Innovation",  wait_ms: 1500 });
        h.insert("name of the elements", Action { name: "Name of the Elements",  wait_ms: 1500 });
        h.insert("reuse", Action { name: "Reuse",  wait_ms: 1500 });
        h.insert("waste not ii", Action { name: "Waste Not II",  wait_ms: 1500 });
        h.insert("waste not", Action { name: "Waste Not",  wait_ms: 1500 });
        // Progress Actions
        h.insert("basic synthesis", Action { name: "Basic Synthesis",  wait_ms: 2500 });
        h.insert("brand of the elements", Action { name: "Brand of the Elements",  wait_ms: 2500 });
        h.insert("careful synthesis", Action { name: "Careful Synthesis",  wait_ms: 2500 });
        h.insert("focused synthesis", Action { name: "Focused Synthesis",  wait_ms: 2500 });
        h.insert("intensive synthesis", Action { name: "Intensive Synthesis",  wait_ms: 2500 });
        h.insert("muscle memory", Action { name: "Muscle Memory",  wait_ms: 2500 });
        h.insert("rapid synthesis", Action { name: "Rapid Synthesis",  wait_ms: 2500 });
        // Quality Actions
        h.insert("basic touch", Action { name: "Basic Touch",  wait_ms: 2500 });
        h.insert("byregot's blessing", Action { name: "Byregot's Blessing",  wait_ms: 2500 });
        h.insert("focused touch", Action { name: "Focused Touch",  wait_ms: 2500 });
        h.insert("hasty touch", Action { name: "Hasty Touch",  wait_ms: 2500 });
        h.insert("patient touch", Action { name: "Patient Touch",  wait_ms: 2500 });
        h.insert("precise touch", Action { name: "Precise Touch",  wait_ms: 2500 });
        h.insert("prudent touch", Action { name: "Prudent Touch",  wait_ms: 2500 });
        h.insert("reflect", Action { name: "Reflect",  wait_ms: 2500 });
        h.insert("standard touch", Action { name: "Standard Touch",  wait_ms: 2500 });
        h.insert("trained eye", Action { name: "Trained Eye",  wait_ms: 2500 });
        // Repair Actions
        h.insert("manipulation", Action { name: "Manipulation",  wait_ms: 1500 });
        h.insert("master's mend", Action { name: "Master's Mend",  wait_ms: 2500 });
        // Other Actions
        h.insert("delicate synthesis", Action { name: "Delicate Synthesis",  wait_ms: 2500 });
        h.insert("observe", Action { name: "Observe",  wait_ms: 2500 });
        h.insert("tricks of the trade", Action { name: "Tricks of the Trade",  wait_ms: 2500 });
        h
    };
}

pub fn from_str(s: &str) -> anyhow::Result<()> {
    let des = toml::from_str::<MacroFileToml>(s)?;
    let mut parsed_vec: Vec<Macro> = Vec::new();
    for macro_toml in &des.xiv_macro {
        parsed_vec.push(Macro {
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

    MACROS.set(parsed_vec).expect("couldn't set up macro cache");
    Ok(())
}

// Attempts to parse macros in |buffer| and return a list of actions.
fn parse_buffer(buffer: &str) -> anyhow::Result<Vec<&'static Action>> {
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

// Extract the action for a given line in a macro. Returns a
// String in the event of an error indicating a malformed macros.
pub fn parse_line(line: &str) -> anyhow::Result<&'static Action> {
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
                log::trace!("\"{}\": m_min {} > rlvl {}", m.name, m_min, rlvl);
                continue;
            }
        }

        if let Some(m_max) = m.max_rlvl {
            if m_max < rlvl {
                log::trace!("\"{}\": m_max {} < rlvl {}", m.name, m_max, rlvl);
                continue;
            }
        }

        // Match on difficulty if it exists
        if let Some(m_difficulty) = m.difficulty {
            if m_difficulty != difficulty {
                log::trace!(
                    "\"{}\": m_difficulty {} != difficulty {}",
                    m.name,
                    m_difficulty,
                    difficulty
                );
                continue;
            }
        }

        if m.specialist && m.specialist != specialist {
            log::trace!(
                "\"{}\": m_specialist {} != specialist {}",
                m.name,
                m.specialist,
                specialist
            );
            continue;
        }

        // At this point check if the recipe durability exists in the macro's
        // durability list.
        if m.durability.iter().any(|&d| d == durability) {
            return i;
        }
    }
    macros().len() - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MACRO_BUFFER: &str = r#"
        /ac "Inner Quiet" <wait.2>
        /ac "Great Strides" <wait.2>
        /ac "Manipulation" <wait.3>
        /ac "Byregot's Blessing" <wait.3>
        #/ac "Commented Action" <wait.1>
        /ac "Careful Synthesis" <wait.3>"#;

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

        let m = super::from_str(MACRO_BUFFER);
        if !m.is_ok() {
            println!("m failure: {:#?}", m);
        }
        assert!(m.is_ok());
        assert!(super::get_macro_for_recipe(35, 0, 0, false) == 0);
        assert!(super::get_macro_for_recipe(40, 0, 0, false) == 2);
        assert!(super::get_macro_for_recipe(60, 0, 0, false) == 0);
        assert!(super::get_macro_for_recipe(70, 0, 0, false) == 2);
        assert!(super::get_macro_for_recipe(80, 0, 0, false) == 1);
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

    #[test]
    fn macros_buffer() -> anyhow::Result<()> {
        let actual = parse_buffer(TEST_MACRO_BUFFER)?;
        assert_eq!(validate_test_entries(actual), true);
        Ok(())
    }

    fn validate_test_entries(actual: Vec<&Action>) -> bool {
        let expected = [
            Action {
                name: "Inner Quiet",
                wait_ms: 1500,
            },
            Action {
                name: "Great Strides",
                wait_ms: 1500,
            },
            Action {
                name: "Manipulation",
                wait_ms: 1500,
            },
            Action {
                name: "Byregot's Blessing",
                wait_ms: 2500,
            },
            Action {
                name: "Careful Synthesis",
                wait_ms: 2500,
            },
        ];

        for (left, &right) in expected.iter().zip(actual.iter()) {
            assert_eq!(left, right);
        }
        true
    }
}
