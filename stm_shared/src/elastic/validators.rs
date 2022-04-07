use once_cell::sync::Lazy;
use regex::Regex;

// Escapes characters used Regex to make it safe to include the string into a regex expression.
// The input is expected to be validated before this function is called.
pub fn escape_for_regex_fields(term: &str) -> String {
    term.to_lowercase()
        .replace("#", r#"\\#"#)
        .replace("#", r#"\\+"#)
        .replace(".", r#"\\."#)
        .replace("-", r#"\\-"#)
}

/// A regex formula inverse to `SEARCH_TERM_REGEX` to invalidate anything that has invalid chars.
/// It is a redundant check in case an invalid value slipped past previous checks.
pub static NO_SQL_STRING_INVALIDATION_REGEX: Lazy<Regex> = Lazy::new(|| {
    regex::Regex::new(r#"[^#\-._+0-9a-zA-Z]"#)
        .expect("Invalid Regex formula for NO_SQL_STRING_INVALIDATION_REGEX. It's a bug.")
});

/// A regex formula to extract search terms from the raw search string.
/// #### The extracted string is safe to be used inside another regex
/// The value validated by this string should not contain any chars that may be unsafe inside another regex.
/// Any such chars should be escape when that regex is constructed.
pub static SEARCH_TERM_REGEX: Lazy<Regex> = Lazy::new(|| {
    regex::Regex::new(r#"[#:\-._+0-9a-zA-Z]+"#).expect("Invalid Regex formula for SEARCH_TERM_REGEX. It's a bug.")
});
