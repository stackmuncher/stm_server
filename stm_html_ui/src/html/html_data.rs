use crate::config::Config;
use crate::html::stats::Stats;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A common data format fed to Tera templates
#[derive(Serialize)]
pub(crate) struct HtmlData {
    /// Raw ES response with dev idx docs
    pub devs: Option<Value>,
    /// A list of all stack technologies with their doc counts
    /// {"aggregations" : {
    ///     "agg" : {
    ///     "doc_count_error_upper_bound" : 0,
    ///     "sum_other_doc_count" : 0,
    ///     "buckets" : [
    ///         {
    ///         "key" : "markdown",
    ///         "doc_count" : 1746219
    ///         }]}}}
    pub stack_stats: Option<Value>,
    /// List of related libraries, fully qualified  
    pub related: Option<Vec<RelatedKeywords>>,
    /// The raw search string as entered by the user
    pub raw_search: String,
    /// List of keywords extracted from the raw search
    pub keywords: Vec<String>,
    /// All search terms from the raw search with their counts from different fields in ES
    pub keywords_meta: Vec<KeywordMetadata>,
    /// A list of search terms matching known languages with minimum number of LoC per lang
    pub langs: Vec<(String, usize)>,
    /// Same as `keywords` as a single string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords_str: Option<String>,
    /// A normalized version of the user login for dev profile page title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_str: Option<String>,
    /// `owner_id` for the dev, if known. For registered devs only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id_str: Option<String>,
    /// Name of the HTML template to use. Defaults to 404
    pub template_name: String,
    /// Time to live for the HTTP response
    pub ttl: u32,
    /// HTTP response code
    pub http_resp_code: u32,
    /// Contents of HTML meta-tag for bots (nofollow, noindex), if any
    /// e.g. `<meta name="robots" content="noindex">` for `rust+actix` search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta_robots: Option<String>,
    /// A container for job stats data populated for stats page only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats_jobs: Option<Stats>,
    /// A copy of API GW request headers
    #[serde(skip)]
    pub headers: HashMap<String, String>,
    /// Timestamp when the request was initiated
    #[serde(skip)]
    pub timestamp: DateTime<Utc>,
    /// Name of the timezone the availability is provided for, e.g. `UTC+08`
    /// The value is taken from the query.
    pub availability_tz: Option<String>,
    /// Minimum number of hours of availability required in the specified timezone.
    /// The value is taken from the query.
    pub availability_tz_hrs: Option<usize>,
    /// Page number in paginated results, defaults to 0
    pub page_number: usize,
    /// Results from ... derived from the page number
    pub results_from: usize,
    /// The maximum number of dev listings per page of search results
    pub devs_per_page: usize,
    /// Max number of pages allowed in search results. There may be fewer results than this value.
    pub max_pages: usize,
    /// List of languages for search
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub all_langs: Vec<String>,
}

/// A view of the keyword from ElasticSearch
#[derive(Serialize)]
pub(crate) struct KeywordMetadata {
    /// A normalized version of what the user searched for
    pub search_term: String,
    /// Language terms can be qualified by min number of lines of code
    /// E.g. `rust:2000`, which should be split into `rust` and `2000`
    pub search_term_loc: usize,
    /// Number of developers using this keyword
    pub es_keyword_count: usize,
    /// Number of developers using this package
    pub es_package_count: usize,
    /// Number of developers using this language
    pub es_language_count: usize,
    /// True if the term got no matches at all. Needed to simplify the front-end logic.
    pub unknown: bool,
    /// True if the number of allowed search terms was exceeded. Needed to simplify the front end
    /// and keen the control of the number in one place.
    pub too_many: bool,
    /// A better formatted term, e.g. `rust` -> `Rust`
    pub search_term_fmt: String,
}

impl KeywordMetadata {
    /// Returns a better formatted name if found in the dictionary, otherwise returns a clone of the same value
    pub(crate) fn format_lang(lang: &String, config: &Config) -> String {
        // this value should already be lower-case
        let lang_lowercase = lang.to_lowercase();

        // find a matching language in the dictionary
        for v in &config.all_langs {
            if lang_lowercase == v.to_lowercase() {
                return v.clone();
            }
        }

        // no match was found
        lang.clone()
    }
}

/// List of related keywords extracted from ES
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct RelatedKeywords {
    pub k: String,
    pub c: u64,
}
