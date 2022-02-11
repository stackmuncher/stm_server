use serde::Deserialize;

// HITS WRAPPER **************************************************************************************

/// An inner member
#[derive(Deserialize, Debug)]
pub struct ESSourceSource<T> {
    #[serde(rename(deserialize = "_source"))]
    pub source: T,
}

/// An inner member
#[derive(Deserialize, Debug)]
pub struct ESHits<T> {
    pub hits: Vec<ESSourceSource<T>>,
    // there should be member `total` with the counts from ESHitsCountHits
    // e.g. "total" : {
    //   "value" : 1,
    //   "relation" : "eq"
    // },
}

// MISC REPORT FIELDS **************************************************************************

/// An inner member
#[derive(Deserialize, Debug)]
pub struct ESReportTimestampTimestamp {
    pub timestamp: String,
}

/// Contains several levels to get to the report's timestamp.
/// To be used as <T> for ESHits.
/// ```json
///"report" : {
///  "timestamp" : "2021-03-08T20:11:05.862454103+00:00"
///}
/// ```
#[derive(Deserialize, Debug)]
pub struct ESReportTimestamp {
    pub report: ESReportTimestampTimestamp,
}

/// Member of ESHitsCount
#[derive(Deserialize)]
pub struct ESHitsCountTotals {
    pub value: usize,
}

/// Member of ESHitsCount
#[derive(Deserialize)]
pub struct ESHitsCountHits {
    pub total: ESHitsCountTotals,
}

/// Corresponds to ES response metadata
/// ```json
/// {
///     "took" : 652,
///     "timed_out" : false,
///     "_shards" : {
///         "total" : 5,
///         "successful" : 5,
///         "skipped" : 0,
///         "failed" : 0
///     },
///     "hits" : {
///         "total" : {
///         "value" : 0,
///         "relation" : "eq"
///         },
///         "max_score" : null,
///         "hits" : [ ]
///     }
/// }
/// ```
#[derive(Deserialize)]
pub struct ESHitsCount {
    pub hits: ESHitsCountHits,
}

/// Top level contents of _source
/// To be used as <T> for ESHits.
/// ```json
/// "_source" : {
///     "login" : "MarkStefanovic",
///     "id" : 13571999,
///     "node_id" : "MDQ6VXNlcjEzNTcxOTk5",
///     "avatar_url" : "https://avatars.githubusercontent.com/u/13571999?v=4",
///     "name" : "Mark Stefanovic",
///     "company" : null,
///     "blog" : "",
///     "location" : "US",
///     "email" : null,
///     "hireable" : null,
///     "bio" : null,
///     "twitter_username" : null,
///     "public_repos" : 18,
///     "public_gists" : 0,
///     "followers" : 2,
///     "following" : 0,
///     "created_at" : "2015-07-30T12:56:48Z",
///     "updated_at" : "2021-07-13T10:29:00Z",
///}
/// ```
#[derive(Deserialize, Debug)]
pub struct ESSourceDev {
    /// GH login
    pub login: Option<String>,
    /// An ISO3389 timestamp of when the gh a/c was created (from GH)
    /// e.g. 2013-11-13T05:06:37Z
    pub created_at: Option<String>,
    /// Public email address from GH
    pub email: Option<String>,
    /// A free-text location from GH
    pub location: Option<String>,
}