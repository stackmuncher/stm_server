use super::{Ctx, Query};
use crate::elastic;
use juniper::{graphql_object, FieldResult};
use stm_shared::elastic as elastic_shared;
use stm_shared::graphql::RustScalarValue;
use tracing::error;

// This block has to contain all queries for the macro to work. It is possible to split it into multiple modules
// with a bit of a workaround. See https://github.com/graphql-rust/juniper/discussions/1045
#[graphql_object(context=Ctx, scalar = RustScalarValue)]
impl Query {
    /// Returns a list of all `tech` names with doc counts.
    /// See stm_graphql/samples/es-responses/devs_per_language.json for the full example.
    async fn devs_per_language<'db>(&self, context: &'db Ctx) -> FieldResult<elastic_shared::types::ESAggs> {
        // get number of devs per technology
        let stack_stats = match elastic_shared::search::<elastic_shared::types::ESAggs>(
            &context.es_url,
            &context.dev_idx,
            Some(elastic::QUERY_DEVS_PER_TECH),
        )
        .await
        {
            Ok(v) => v,
            Err(_) => return Err("ES query failed. See server logs.".into()),
        };

        Ok(stack_stats)
    }

    /// Returns the number of devs matching the stack.
    /// See stm_graphql/samples/es-responses/dev_count_for_stack.json for the full example.
    async fn dev_count_for_stack<'db>(
        &self,
        context: &'db Ctx,
        stack: Vec<elastic::TechExperience>,
    ) -> FieldResult<i32> {
        // get number of devs per technology
        let dev_count = match elastic::matching_dev_count(
            &context.es_url,
            &context.dev_idx,
            Vec::new(),
            stack,
            0,
            0,
            &context.no_sql_string_invalidation_regex,
        )
        .await
        {
            Ok(v) => v,
            Err(_) => return Err("ES query failed. See server logs.".into()),
        };

        let dev_count = match serde_json::from_value::<elastic_shared::types::ESDocCount>(dev_count) {
            Ok(v) => v.count,
            Err(e) => {
                error!("Failed to convert dev_count response with {e}");
                return Err("ES query failed. See server logs.".into());
            }
        };

        Ok(dev_count)
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

    std::fs::write("samples/gql-responses/devsPerLanguage.gql.json", gql_data.clone())
        .expect("Unable to write 'samples/gql-responses/devsPerLanguage.gql.json' file");

    assert!(result.is_ok(), "devs_per_language query executed with errors");
    assert!(
        gql_data.len() > 1000,
        "devs_per_language query response is too short {}. Expecting at least 1000 chars.",
        gql_data.len()
    );
}

#[tokio::test]
async fn dev_count_for_stack() {
    let config = super::Config::new();

    let gql_request = super::GraphQLRequest::<super::RustScalarValue> {
        query: r#"query { devCountForStack (langs: [{tech: "rust"}])}"#.to_string(),
        operation_name: None,
        variables: None,
    };

    let (gql_data, result) = super::execute_gql(&config, gql_request).await.unwrap();

    std::fs::write("samples/gql-responses/devCountForStack.gql.json", gql_data.clone())
        .expect("Unable to write 'samples/gql-responses/devCountForStack.gql.json' file");

    assert!(result.is_ok(), "devCountForStack query executed with errors");
    assert!(
        regex::Regex::new(r#"\{"data":\{"devCountForStack":\d+\}\}"#)
            .unwrap()
            .is_match(&gql_data),
        "Unexpected devCountForStack query response"
    );
}
