use super::{Ctx, Query};
use crate::elastic;
use juniper::{graphql_object, FieldResult};
use stm_shared::elastic as elastic_shared;
use stm_shared::graphql::RustScalarValue;

#[graphql_object(context=Ctx, scalar = RustScalarValue)]
impl Query {
    /// Returns a list of all `tech` names with doc counts.
    /// ```
    /// "aggregations" : {
    /// "agg" : {
    ///     "doc_count_error_upper_bound" : 0,
    ///     "sum_other_doc_count" : 0,
    ///     "buckets" : [
    ///       {
    ///         "key" : "markdown",
    ///         "doc_count" : 1819766
    ///       },
    /// ```
    /// See stm_graphql/samples/sample-responses/devs_per_language.json for the full example.
    async fn devs_per_language<'db>(&self, context: &'db Ctx) -> FieldResult<elastic_shared::types::ESAggs> {
        // get number of devs per technology
        let stack_stats = match elastic_shared::search::<elastic_shared::types::ESAggs>(
            &context.es_url,
            &context.dev_idx,
            Some(elastic::SEARCH_ALL_LANGUAGES),
        )
        .await
        {
            Ok(v) => v,
            Err(_) => return Err("ES query failed. See server logs.".into()),
        };

        Ok(stack_stats)
    }
}

#[tokio::test]
async fn devs_per_language() {
    let config = super::Config::new();

    let gql_request = super::GraphQLRequest::<super::RustScalarValue> {
        query: r#"query { devsPerLanguage { aggregations {agg {buckets {key, docCount}}} }}"#.to_string(),
        operation_name: None,
        variables: None,
    };

    let (gql_data, result) = super::execute_gql(&config, gql_request).await.unwrap();
    assert!(result.is_ok(), "devs_per_language query executed with errors");
    assert!(
        gql_data.len() > 1000,
        "devs_per_language query response is too short {}. Expecting at least 1000 chars.",
        gql_data.len()
    );
}
