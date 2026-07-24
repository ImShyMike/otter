use std::str::FromStr;

use celes::{Country, LookupTable};
use strsim::jaro_winkler;
use unicode_normalization::UnicodeNormalization;

/// Endonyms and abbreviations
const ALIASES: &[(&str, &str)] = &[
    // endonyms
    ("deutschland", "DE"),
    ("espana", "ES"),
    ("espanya", "ES"),
    ("polska", "PL"),
    ("magyarorszag", "HU"),
    ("osterreich", "AT"),
    ("rakusko", "AT"),
    ("suomi", "FI"),
    ("sverige", "SE"),
    ("norge", "NO"),
    ("danmark", "DK"),
    ("nederland", "NL"),
    ("lietuva", "LT"),
    ("suisse", "CH"),
    ("schweiz", "CH"),
    ("svizzera", "CH"),
    ("maroc", "MA"),
    ("cesko", "CZ"),
    ("ceska republika", "CZ"),
    ("slovensko", "SK"),
    ("brasil", "BR"),
    ("italia", "IT"),
    ("estados unidos", "US"),
    ("soedinennye shtaty", "US"),
    ("соединенные штаты", "US"),
    ("대한민국", "KR"),
    ("中国", "CN"),
    ("中國", "CN"),
    ("台灣", "TW"),
    ("台湾", "TW"),
    ("日本", "JP"),
    ("加拿大", "CA"),
    ("مصر", "EG"),
    ("ελλάδα", "GR"),
    ("aland", "AX"),
    // abbreviations
    ("uk", "GB"),
    ("uae", "AE"),
    ("usa", "US"),
    ("us of a", "US"),
    ("korea", "KR"),
    ("korea south", "KR"),
    ("south korea", "KR"),
    ("korea north", "KP"),
    ("moldova republic of", "MD"),
    ("china hksar", "HK"),
    ("hksar", "HK"),
    ("hong kong sar", "HK"),
    ("hong kong sar china", "HK"),
    ("bosna i herzegovina", "BA"),
    ("bosna i hercegovina", "BA"),
    ("republica dominicana", "DO"),
    ("republic democratic of congo", "CD"),
    ("democratic republic of congo", "CD"),
    ("congo drc", "CD"),
    // constituent countries of the UK
    ("england", "GB"),
    ("scotland", "GB"),
    ("wales", "GB"),
    ("northern ireland", "GB"),
    ("england uk", "GB"),
    ("england united kingdom", "GB"),
];

/// Placeholder junk values
const JUNK: &[&str] = &["n/a", "none", "other", "unknown", "redacted"];

fn strip_article(s: &str) -> &str {
    s.strip_prefix("the ").unwrap_or(s)
}

/// Normalize text into a lowercased ASCII
pub fn normalize(s: &str) -> String {
    let deaccented: String = s
        .nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect();

    deaccented
        .nfc()
        .collect::<String>()
        .to_lowercase()
        .replace('.', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Drops parenthesised asides and any non-alphabetic character
fn clean(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0usize;

    for c in s.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ if depth > 0 => {}
            _ if c.is_alphabetic() => out.push(c),
            _ if c.is_whitespace() => out.push(' '),
            _ => {}
        }
    }

    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Detects fields that have digits in the original text but almost no letters left after cleaning
fn looks_like_postal_code(raw_norm: &str, cleaned: &str) -> bool {
    let has_digit = raw_norm.chars().any(|c| c.is_ascii_digit());
    let alpha_count = cleaned.chars().filter(|c| c.is_alphabetic()).count();
    has_digit && alpha_count <= 4
}

fn code_lookup(token: &str) -> Option<String> {
    let upper = token.to_uppercase();
    if upper.len() != 2 && upper.len() != 3 {
        return None;
    }

    Country::get_countries().iter().find_map(|c| {
        if c.alpha2.eq_ignore_ascii_case(&upper) || c.alpha3.eq_ignore_ascii_case(&upper) {
            Some(c.alpha2.to_string())
        } else {
            None
        }
    })
}

/// Resolves a country name or alias to its alpha-2 code
fn resolve_named(target: &str) -> Option<String> {
    if let Some((_, code)) = ALIASES.iter().find(|(name, _)| *name == target) {
        return Some((*code).to_string());
    }

    // space separated first, then squashed
    if let Ok(c) = Country::from_str(target) {
        return Some(c.alpha2.to_string());
    }
    if let Ok(c) = Country::from_str(&target.replace(' ', "")) {
        return Some(c.alpha2.to_string());
    }
    None
}

/// Full resolution for a standalone segment of text
fn resolve_segment(raw_norm: &str) -> Option<String> {
    let cleaned = clean(raw_norm);
    if cleaned.is_empty() {
        return None;
    }
    if looks_like_postal_code(raw_norm, &cleaned) {
        return None;
    }

    let target = strip_article(&cleaned);

    resolve_named(target).or_else(|| code_lookup(&cleaned.replace(' ', "")))
}

/// Resolves a country name or code to its alpha-2 code, falling back to fuzzy matching
pub fn resolve_country(raw: &Option<String>) -> Option<String> {
    let norm = normalize(raw.as_ref()?);
    if JUNK.contains(&norm.as_str()) {
        return None;
    }

    // composite entries
    for part in norm.split(['/', ',', '|', '&']) {
        if let Some(code) = resolve_segment(part) {
            return Some(code);
        }
    }

    let cleaned = clean(&norm);
    if cleaned.is_empty() {
        return None;
    }
    if looks_like_postal_code(&norm, &cleaned) {
        return None;
    }

    // a country name buried in a longer sentence, one word at a time
    for word in cleaned.split_whitespace() {
        if word.len() < 3 {
            continue;
        }
        if let Some(code) = resolve_named(word) {
            return Some(code);
        }
    }

    let target = strip_article(&cleaned);
    if target.len() < 5 {
        return None;
    }

    let squashed = target.replace(' ', "");

    // fuzzy fallback
    Country::get_countries()
        .iter()
        .map(|c| {
            let long = c.long_name.to_lowercase();
            let candidates = std::iter::once(strip_article(&long).to_string())
                .chain(c.aliases.iter().map(|a| a.to_lowercase()));

            let best = candidates
                .map(|cand| jaro_winkler(target, &cand).max(jaro_winkler(&squashed, &cand)))
                .fold(0.0_f64, f64::max);

            (c, best)
        })
        .filter(|(_, score)| *score > 0.9)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(c, _)| c.alpha2.to_string())
}
