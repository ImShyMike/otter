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

/// Resolves a country name or code to its alpha-2 code, falling back to fuzzy matching
pub fn resolve_country(raw: &Option<String>) -> Option<String> {
    let norm = normalize(raw.as_ref()?);
    if JUNK.contains(&norm.as_str()) {
        return None;
    }

    let norm = clean(&norm);
    if norm.is_empty() {
        return None;
    }

    let target = strip_article(&norm);

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

    // fuzzy fallback
    if target.len() < 5 {
        return None;
    }

    let squashed = target.replace(' ', "");

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
