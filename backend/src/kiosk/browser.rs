//! Port of Rails 8.0.2 `allow_browser versions: :modern`.
//!
//! Rails blocks a request with 406 when the `useragent` gem reports a browser
//! from the `:modern` table below a minimum version, unless it is a bot or no
//! version could be read. This reproduces the gem's detection for the
//! browsers that table names; every case in the tests was captured from the
//! Rails app. Browsers the table does not name (Edge legacy, Vivaldi, curl,
//! unknown agents) are always allowed, as in Rails.

/// `ActionController::AllowBrowser::BrowserBlocker::SETS[:modern]`.
/// `None` means always blocked (Internet Explorer).
const MODERN: &[(&str, Option<&str>)] = &[
    ("safari", Some("17.2")),
    ("chrome", Some("120")),
    ("firefox", Some("121")),
    ("opera", Some("106")),
    ("ie", None),
];

/// True when Rails would answer 406 Not Acceptable for this `User-Agent`.
pub fn is_blocked(user_agent: Option<&str>) -> bool {
    let Some(ua) = user_agent.map(str::trim).filter(|u| !u.is_empty()) else {
        return false;
    };
    if is_bot(ua) {
        return false;
    }
    let (name, version) = detect(ua);
    if version.is_empty() {
        return false;
    }
    match MODERN.iter().find(|(n, _)| *n == name) {
        Some((_, Some(min))) => version_lt(&version, min),
        Some((_, None)) => true,
        None => false,
    }
}

/// The gem's bot test: crawler-ish words anywhere in the agent.
fn is_bot(ua: &str) -> bool {
    let lower = ua.to_ascii_lowercase();
    ["bot", "crawl", "spider", "slurp"].iter().any(|w| lower.contains(w))
}

/// Version after `token` (`"Chrome/"`), only where the token starts a word, so
/// `HeadlessChrome/` is not read as `Chrome/`.
fn token_version(ua: &str, token: &str) -> Option<String> {
    let mut from = 0;
    while let Some(rel) = ua[from..].find(token) {
        let at = from + rel;
        let starts_word = at == 0 || matches!(ua.as_bytes()[at - 1], b' ' | b'(' | b';');
        if starts_word {
            let rest = &ua[at + token.len()..];
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.'))
                .unwrap_or(rest.len());
            return Some(rest[..end].to_string());
        }
        from = at + token.len();
    }
    None
}

/// `(browser, version)` as the `useragent` gem names them (lowercased).
fn detect(ua: &str) -> (&'static str, String) {
    if let Some(v) = token_version(ua, "MSIE ") {
        return ("ie", v);
    }
    if ua.contains("Trident/") {
        return ("ie", token_version(ua, "rv:").unwrap_or_default());
    }
    if let Some(v) = token_version(ua, "Edge/") {
        return ("edge", v);
    }
    if let Some(v) = token_version(ua, "OPR/") {
        return ("opera", v);
    }
    if ua.starts_with("Opera/") {
        return ("opera", token_version(ua, "Version/").unwrap_or_default());
    }
    if let Some(v) = token_version(ua, "Vivaldi/") {
        return ("vivaldi", v);
    }
    if let Some(v) = token_version(ua, "CriOS/").or_else(|| token_version(ua, "Chrome/")) {
        return ("chrome", v);
    }
    if let Some(v) = token_version(ua, "Firefox/") {
        return ("firefox", v);
    }
    if ua.contains("Safari") && ua.contains("AppleWebKit") {
        return ("safari", safari_version(ua));
    }
    let first = ua.split_whitespace().next().unwrap_or("");
    let (_, v) = first.split_once('/').unwrap_or((first, ""));
    ("other", v.to_string())
}

/// Safari's `Version/x`, or on iOS without it the OS version (`OS 16_6`).
fn safari_version(ua: &str) -> String {
    if let Some(v) = token_version(ua, "Version/") {
        return v;
    }
    if ua.contains("iPhone") || ua.contains("iPad") || ua.contains("iPod") {
        if let Some(at) = ua.find(" OS ") {
            let rest = &ua[at + 4..];
            let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '_')).unwrap_or(rest.len());
            return rest[..end].replace('_', ".");
        }
    }
    String::new()
}

/// Segment-wise numeric compare (`UserAgent::Version#<`).
fn version_lt(have: &str, min: &str) -> bool {
    let parts = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|p| p.chars().take_while(char::is_ascii_digit).collect::<String>())
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    let (a, b) = (parts(have), parts(min));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x < y;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (blocked, user agent) captured from Rails 8.0.2 `BrowserBlocker#blocked?`.
    const MATRIX: &[(bool, &str)] = &[
        (true, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36"),
        (false, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"),
        (true, "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0"),
        (false, "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0"),
        (true, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15"),
        (false, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15"),
        (true, "Mozilla/5.0 (iPhone; CPU iPhone OS 16_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Mobile/15E148 Safari/604.1"),
        (true, "Mozilla/5.0 (iPhone; CPU iPhone OS 16_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/80.0.0.0 Mobile/15E148 Safari/604.1"),
        (true, "Mozilla/5.0 (iPhone; CPU iPhone OS 16_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) FxiOS/100.0 Mobile/15E148 Safari/605.1.15"),
        (true, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.0.0 Safari/537.36 Edg/80.0.0.0"),
        (true, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.0.0 Safari/537.36 OPR/90.0.0.0"),
        (true, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 OPR/105.0.0.0"),
        (true, "Mozilla/5.0 (Linux; Android 13; SM-S901B) AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/20.0 Chrome/106.0.0.0 Mobile Safari/537.36"),
        (true, "Mozilla/5.0 (Windows NT 10.0; Trident/7.0; rv:11.0) like Gecko"),
        (true, "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; Trident/5.0)"),
        (false, "Mozilla/5.0 (Linux; Android 6.0.1; Nexus 5X Build/MMB29P) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.0.0 Mobile Safari/537.36 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)"),
        (true, "Mozilla/5.0 (Linux; Android 10; K; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/80.0.0.0 Mobile Safari/537.36"),
        (false, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/80.0.0.0 Safari/537.36"),
        (true, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.0.0 YaBrowser/22.1.0 Safari/537.36"),
        (false, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.0.0 Safari/537.36 Edge/18.0"),
        (true, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chromium/80.0.0.0 Chrome/80.0.0.0 Safari/537.36"),
        (true, "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:80.0) Gecko/20100101 Firefox/80.0"),
        (false, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Safari/605.1.15"),
        (true, "Opera/9.80 (Windows NT 6.1; U; en) Presto/2.12.388 Version/12.16"),
        (true, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.6045.105 Electron/28.0.0 Safari/537.36"),
        (true, "Mozilla/5.0 (Linux; Android 13) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.6045.163 Mobile Safari/537.36"),
        (true, "Mozilla/5.0 (iPad; CPU OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1"),
        (false, "curl/8.0"),
        (false, "Mozilla/5.0"),
        (true, "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0 SeaMonkey/2.53.17"),
        (true, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119 Safari/537.36"),
        (false, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36 Vivaldi/6.0"),
    ];

    #[test]
    fn matches_rails_matrix() {
        for (blocked, ua) in MATRIX {
            assert_eq!(is_blocked(Some(ua)), *blocked, "{ua}");
        }
    }

    #[test]
    fn missing_user_agent_is_allowed() {
        assert!(!is_blocked(None));
    }

    #[test]
    fn blank_user_agent_is_allowed() {
        assert!(!is_blocked(Some("  ")));
    }

    #[test]
    fn version_compare_is_numeric_per_segment() {
        assert!(version_lt("119.0.6045.105", "120"));
        assert!(!version_lt("120.0.0.0", "120"));
        assert!(version_lt("17.1", "17.2"));
        assert!(!version_lt("17.10", "17.2"));
    }
}
