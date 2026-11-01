//! Text folding shared by the kernel and the modules: names, organisations and search terms
//! compare without case, accents, đ or spacing (D-022, D-028).

use unicode_normalization::UnicodeNormalization;

/// "  Nguyễn  Văn Tiến " → "nguyen van tien": case, accents, đ and spacing don't matter.
pub fn fold(text: &str) -> String {
    let stripped: String = text
        .nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .map(|c| match c {
            'đ' | 'Đ' => 'd',
            c => c,
        })
        .collect();
    stripped
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
