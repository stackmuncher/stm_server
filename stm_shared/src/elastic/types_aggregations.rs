use serde::Deserialize;

/// A generic structure for ES aggregations result. Make sure the aggregation name is `agg`.
/// ```json
///   {
///     "aggregations" : {
///       "agg" : {
///         "buckets" : [
///           {
///             "key" : "twilio",
///             "doc_count" : 597
///           }
///         ]
///       }
///     }
///   }
/// ```
#[derive(Deserialize)]
pub struct ESAggs {
    pub aggregations: ESAggsAgg,
}

/// Part of ESAggs
#[derive(Deserialize)]
pub struct ESAggsBucket {
    pub key: String,
    pub doc_count: usize,
}

/// Part of ESAggs
#[derive(Deserialize)]
pub struct ESAggsBuckets {
    pub buckets: Vec<ESAggsBucket>,
}

/// Part of ESAggs
#[derive(Deserialize)]
pub struct ESAggsAgg {
    pub agg: ESAggsBuckets,
}
