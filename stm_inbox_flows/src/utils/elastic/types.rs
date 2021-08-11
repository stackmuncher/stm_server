use serde::Deserialize;

// HITS WRAPPER **************************************************************************************

/// An inner member
#[derive(Deserialize, Debug)]
pub(crate) struct ESSourceSource<T> {
    #[serde(rename(deserialize = "_source"))]
    pub source: T,
}

/// An inner member
#[derive(Deserialize, Debug)]
pub(crate) struct ESHits<T> {
    pub hits: Vec<ESSourceSource<T>>,
}

/// A generic wrapper to get to any type of _source in ES response. E.g.
/// ```json
/// {
///   "hits" : {
///     "hits" : [
///       {
///         "_source" : {
///           "report" : {
///             "timestamp" : "2021-03-08T20:11:05.862454103+00:00"
///           }
///         }
///       }
///     ]
///   }
/// }
/// ```
#[derive(Deserialize, Debug)]
pub(crate) struct ESSource<T> {
    pub hits: ESHits<T>,
}


// MISC REPORT FIELDS **************************************************************************

/// An inner member
#[derive(Deserialize, Debug)]
pub(crate) struct ESReportTimestampTimestamp {
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
pub(crate) struct ESReportTimestamp {
    pub report: ESReportTimestampTimestamp,
}
