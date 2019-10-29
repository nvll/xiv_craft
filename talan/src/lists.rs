use clipboard::{ClipboardContext, ClipboardProvider};
use failure::{format_err, Error};

// Should use string probably
#[derive(Debug, PartialEq)]
pub struct ListItem {
    pub item: String,
    pub count: u32,
}

pub fn import_tasks_from_clipboard() -> Result<Vec<ListItem>, Error> {
    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    parse_item_list(&ctx.get_contents().unwrap())
}

pub fn parse_item_list(string: &str) -> Result<Vec<ListItem>, Error> {
    log::info!("Reading items from list:");
    let mut v: Vec<ListItem> = Vec::new();
    for line in string.split('\n') {
        let line_trimmed = line.trim();
        if let Ok(r) = parse_list_line(line_trimmed) {
            log::info!("    {}x {}", r.count, r.item);
            v.push(r);
        }
    }
    log::info!("done.");

    Ok(v)
}

fn parse_list_line(line: &str) -> Result<ListItem, Error> {
    // Every item should have {NUM}x {NAME}. If we can't split here, then
    // assume the string is just an item name and count is 1.
    let v: Vec<&str> = line.split("x ").collect();
    if line.is_empty() || !line.chars().nth(0).unwrap().is_ascii_digit() || v.len() < 2 {
        return Err(format_err!("Empty list item!"));
    }

    let mut count = 0;
    if v.len() > 1 {
        // Parse the count out from the first side of the split and adjust where
        // we find the name.
        for c in v[0].chars() {
            if c.is_ascii_digit() {
                count = (count * 10) + c.to_digit(10).unwrap();
            } else {
                break;
            }
        }
    }

    if count == 0 {
        count = 1;
    }
    // At this point there should be an 'x' followed by a space

    Ok(ListItem {
        item: v[1].to_string(),
        count,
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use clipboard::{ClipboardContext, ClipboardProvider};

    const TEST_DATA: &str = "3x ItemName
    0x Item Name
    1000x Item Name";

    const TEST_DATA_RESULTS: [(&'static str, u32); 3] =
        [("ItemName", 3), ("Item Name", 1), ("Item Name", 1000)];

    #[test]
    fn buffer_parse_test() -> Result<(), Error> {
        for (actual, expected) in parse_item_list(TEST_DATA)?
            .iter()
            .zip(TEST_DATA_RESULTS.iter())
        {
            assert_eq!(
                *actual,
                ListItem {
                    item: expected.0.to_string(),
                    count: expected.1
                }
            );
        }

        Ok(())
    }

    #[test]
    fn line_parse_test() -> Result<(), Error> {
        for (i, line) in TEST_DATA.lines().enumerate() {
            let r = parse_list_line(line.trim())?;
            assert_eq!(r.item, TEST_DATA_RESULTS[i].0);
            assert_eq!(r.count, TEST_DATA_RESULTS[i].1);
        }

        Ok(())
    }

    #[test]
    fn line_parse_invalid_test() -> Result<(), Error> {
        assert!(parse_list_line("").is_err());
        assert!(parse_list_line("x name").is_err());
        assert!(parse_list_line("xname").is_err());

        Ok(())
    }

    #[test]
    #[ignore]
    fn clipboard_list_test() -> Result<(), Error> {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        let buf = ctx.get_contents().unwrap();
        let actual = parse_item_list(&buf);
        println!("contents: {:#?}", actual);
        Ok(())
    }
}
