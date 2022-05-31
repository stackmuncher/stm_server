use juniper::GraphQLObject;
use serde::{Deserialize, Serialize};
use stackmuncher_lib::graphql::RustScalarValue;

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
#[derive(Deserialize, GraphQLObject, Serialize)]
#[graphql(scalar = RustScalarValue)]
pub struct ESAggs {
    pub aggregations: ESAggsAgg,
}

/// Part of ESAggs
#[derive(Deserialize, GraphQLObject, Serialize)]
#[graphql(scalar = RustScalarValue)]
pub struct ESAggsBucket {
    pub key: String,
    pub doc_count: u64,
}

/// Part of ESAggs
#[derive(Deserialize, GraphQLObject, Serialize)]
#[graphql(scalar = RustScalarValue)]
pub struct ESAggsBuckets {
    pub buckets: Vec<ESAggsBucket>,
}

/// Part of ESAggs
#[derive(Deserialize, GraphQLObject, Serialize)]
#[graphql(scalar = RustScalarValue)]
pub struct ESAggsAgg {
    pub agg: ESAggsBuckets,
}

impl Default for ESAggs {
    fn default() -> Self {
        serde_json::from_str(r#"{"aggregations" : {"agg" : {"buckets" : [{"key" : "twilio","doc_count" : 597}]}}}"#)
            .unwrap()
    }
}
