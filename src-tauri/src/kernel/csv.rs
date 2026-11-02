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

/// What a spreadsheet would run as a formula when a cell starts with it.
const FORMULA: [char; 6] = ['=', '+', '-', '@', '\t', '\r'];

/// One field, quoted when it needs to be — and, starting like a formula (not a plain number),
/// behind a `'` so a spreadsheet shows it as text (`unguard` takes it off on the way back in).
fn quote(field: &str) -> String {
    let guarded;
    let field = if field.starts_with(FORMULA) && field.trim().parse::<f64>().is_err() {
        guarded = format!("'{field}");
        guarded.as_str()
    } else {
        field
    };
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_owned()
    }
}

/// A cell as it was before `quote` guarded it.
pub fn unguard(cell: &str) -> &str {
    match cell.strip_prefix('\'') {
        Some(rest) if rest.starts_with(FORMULA) => rest,
        _ => cell,
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
    fn a_formula_is_text_out_and_itself_back_in() {
        let rows = vec![vec!["=1+1".to_owned(), "-500".to_owned(), "+84 90 123".to_owned(), "@me".to_owned(), "- follow up".to_owned(), "plain".to_owned()]];
        let text = write(&rows);
        assert_eq!(text, "'=1+1,-500,'+84 90 123,'@me,'- follow up,plain\r\n", "a number stays a number");
        let back: Vec<String> = parse(&text)[0].iter().map(|c| unguard(c).to_owned()).collect();
        assert_eq!(back, rows[0]);
    }

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
