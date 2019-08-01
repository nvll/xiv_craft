use failure::Error;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::{fmt, fs};

#[derive(Debug)]
pub struct MacroFile {
    pub name: String,
    pub path: PathBuf,
    pub actions: Vec<Action>,
}

#[derive(Debug, PartialEq)]
pub struct Action {
    pub name: String,
    pub wait: u64,
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

// Opens |file| and attempts to parse it as a macro list.
pub fn parse_file(macros_file: &Path) -> Result<Vec<Action>, Error> {
    let buffer = fs::read_to_string(macros_file)?;
    parse_buffer(&buffer)
}

pub fn get_macro_list() -> Result<Vec<MacroFile>, failure::Error> {
    let mut v: Vec<MacroFile> = Vec::new();
    for entry in fs::read_dir("macros")? {
        let entry = entry?;
        v.push(MacroFile {
            name: entry.file_name().into_string().unwrap(),
            path: entry.path(),
            actions: parse_file(&entry.path())?,
        });
    }
    Ok(v)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const TEST_MACRO_BUFFER: &str = r#"
        /ac "Comfort Zone" <wait.3>
        /ac "Inner Quiet" <wait.2>
        /ac "Great Strides" <wait.2>
        /ac "Manipulation II" <wait.3>
        /ac "Byregot's Blessing" <wait.3>
        #/ac "Commented Action" <wait.1>
        /ac "Careful Synthesis III" <wait.3>"#;

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

    #[test]
    fn macros_file() -> Result<(), Error> {
        let mut file = NamedTempFile::new()?;
        file.write_all(TEST_MACRO_BUFFER.as_bytes()).unwrap();
        let actual = parse_file(file.path().as_ref())?;
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
