const MAX_OWNER_LEN: usize = 39;
const MAX_REPO_LEN: usize = 100;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedCodeUrl {
    pub is_github: bool,
    pub owner: Option<String>,
    pub repo: Option<String>,
}

/// Parse a raw `code_url` string
pub fn parse_code_url(input: &str) -> ParsedCodeUrl {
    let s = input.trim();
    if s.is_empty() {
        return ParsedCodeUrl::default();
    }

    let Some((host, path)) = split_http_host_and_path(s) else {
        return ParsedCodeUrl::default();
    };

    let segments: Vec<&str> = path.split('/').filter(|seg| !seg.is_empty()).collect();

    let owner = segments.first().and_then(|s| sanitize_owner(s));
    let repo = segments.get(1).and_then(|s| sanitize_repo(s));

    let is_github = {
        let trimmed = host.trim_start_matches("www.").to_ascii_lowercase();
        trimmed == "github.com"
    };

    ParsedCodeUrl {
        is_github,
        owner,
        repo,
    }
}

fn split_http_host_and_path(s: &str) -> Option<(String, String)> {
    let lowered_prefix: String = s.chars().take(8).collect::<String>().to_ascii_lowercase();
    let after_scheme = if lowered_prefix.starts_with("https://") {
        &s[8..]
    } else if lowered_prefix.starts_with("http://") {
        &s[7..]
    } else {
        return None;
    };

    let slash_pos = after_scheme.find('/').unwrap_or(after_scheme.len());
    let after_userinfo = match after_scheme[..slash_pos].rfind('@') {
        Some(i) => &after_scheme[i + 1..],
        None => after_scheme,
    };

    let (host_with_port, path) = match after_userinfo.split_once('/') {
        Some((h, p)) => (h, p),
        None => (after_userinfo, ""),
    };

    let path = path.split(['?', '#']).next().unwrap_or("").to_string();

    let host = host_with_port.split(':').next().unwrap_or("");
    if host.is_empty() {
        return None;
    }

    Some((host.to_string(), path))
}

fn sanitize_owner(raw: &str) -> Option<String> {
    let s = raw.trim().trim_end_matches('/');
    if !is_valid_segment(s, MAX_OWNER_LEN) {
        return None;
    }
    Some(s.to_string())
}

fn sanitize_repo(raw: &str) -> Option<String> {
    let s = raw.trim().trim_end_matches('/');
    let s = s.strip_suffix(".git").unwrap_or(s);
    if !is_valid_segment(s, MAX_REPO_LEN) {
        return None;
    }
    Some(s.to_string())
}

fn is_valid_segment(s: &str, max_len: usize) -> bool {
    if s.is_empty() || s.len() > max_len {
        return false;
    }
    if s == "." || s == ".." {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}
