//! CSV, the standard library only (RFC 4180): quoted fields with commas, quotes and line breaks
//! inside; CRLF or LF; a byte-order mark dropped. Read for imports, written for exports.

/// The rows of a CSV text; every row as it is (ragged rows stay ragged).
pub fn parse(text: &str) -> Vec<Vec<String>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => quoted = false,
                _ => field.push(c),
            }
            continue;
        }
        match c {
            '"' if field.is_empty() => quoted = true,
            ',' => row.push(std::mem::take(&mut field)),
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    // A trailing blank line is not a row.
    rows.retain(|r| !(r.len() == 1 && r[0].is_empty()));
    rows
}

/// One field, quoted when it needs to be.
fn quote(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_owned()
    }
}

/// A CSV text from rows, CRLF line ends (what spreadsheets expect).
pub fn write<R: AsRef<[String]>>(rows: &[R]) -> String {
    let mut out = String::new();
    for row in rows {
        let line: Vec<String> = row.as_ref().iter().map(|f| quote(f)).collect();
        out.push_str(&line.join(","));
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_commas_and_line_breaks_round_trip() {
        let rows = vec![
            vec!["Name".to_owned(), "Note".to_owned()],
            vec!["Keppel, Land".to_owned(), "said \"hi\"\nthen left".to_owned()],
            vec!["Đối tác".to_owned(), String::new()],
        ];
        let text = write(&rows);
        assert_eq!(text, "Name,Note\r\n\"Keppel, Land\",\"said \"\"hi\"\"\nthen left\"\r\nĐối tác,\r\n");
        assert_eq!(parse(&text), rows);
        assert_eq!(parse("\u{feff}a,b\nc,d\n"), vec![vec!["a", "b"], vec!["c", "d"]]);
        assert_eq!(parse("a,b\r\nc"), vec![vec!["a", "b"], vec!["c"]], "a ragged last row without a line end");
        assert!(parse("").is_empty());
    }
}
