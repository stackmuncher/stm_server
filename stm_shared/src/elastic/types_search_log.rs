use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A container for a search results log entry
#[derive(Serialize, Deserialize)]
pub struct SearchLog {
    /// The raw search string as entered by the user
    pub raw: String,
    /// Same as availability_tz in html_data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability_tz: Option<String>,
    /// Same as availability_tz_hrs in html_data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability_tz_hrs: Option<usize>,
    /// List of keywords extracted from the raw search
    pub kw: Vec<String>,
    /// A list of search terms matching known languages
    pub lang: Vec<String>,
    /// Page number of the request, defaults to 1
    #[serde(default)]
    pub page_num: usize,
    /// Source IP address
    pub ip: Option<String>,
    /// EPOCH of the timestamp
    pub ts: i64,
    /// Duration of the request in ms
    pub dur: i64,
    /// List of GH logins found in the response
    pub gh_logins: Vec<String>,
}

impl Hash for SearchLog {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
        self.ip.hash(state);
        self.ts.hash(state);
    }
}

impl SearchLog {
    /// Returns a hash of the object as u64 converted to string
    pub fn get_hash(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish().to_string()
    }
}
