use crate::config::Config;
use crate::elastic;
use juniper::http::{GraphQLRequest, GraphQLResponse};
use juniper::{graphql_object, FieldResult};
use juniper::{EmptyMutation, EmptySubscription, RootNode};
use stm_shared::elastic as elastic_shared;
use stm_shared::elastic::types_aggregations::MyScalarValue;
use tracing::{error, info};

struct Query;

struct Ctx {
    es_url: String,
    dev_idx: String,
}

impl juniper::Context for Ctx {}

#[graphql_object(context=Ctx, scalar = MyScalarValue)]
impl Query {
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
            Err(_) => return Err("error!".into()),
        };

        Ok(stack_stats)
    }
}

pub(crate) async fn execute_gql(config: &Config, gql_request: GraphQLRequest<MyScalarValue>) -> Result<String, ()> {
    info!("Generating vue-home");

    // a struct to be passed to resolvers for accessing backend resources
    let context = Ctx {
        es_url: config.es_url.clone(),
        dev_idx: config.dev_idx.clone(),
    };

    // the GQL schema is static and can be reused between calls
    let root_node =
        RootNode::new_with_scalar_value(Query, EmptyMutation::<Ctx>::new(), EmptySubscription::<Ctx>::new());

    // execute the GQL query from `payload` using GQL resolvers
    let op = gql_request.operation_name.as_deref();
    let vars = &gql_request.variables();
    // let res = crate::execute(&self.query, op, root_node, vars, context).await;
    let res = juniper::execute(&gql_request.query, op, &root_node, vars, &context).await;

    // log any execution errors
    // they will be returned as part of the response to the caller
    if let Err(e) = &res {
        error!("GQL execution error: {}, gql: {:?}", e, gql_request);
    } else {
        for e in &res.as_ref().unwrap().1 {
            error!("GQL field error: {:?}, gql: {:?}", e, gql_request);
        }
    }

    // convert the response into GQL format of {"data": ..., "error": ...} for returning back to the caller over HTTP
    let res = GraphQLResponse::from_result(res);
    match serde_json::to_string(&res) {
        Ok(v) => Ok(v),
        Err(e) => {
            error!("Failed serializing GraphQLResponse with {}", e);
            return Err(());
        }
    }
}

/// Returns the GraphQL schema for the current structures and resolvers.
/// The schema can be in the current working dir at dev time by running `cargo test save_schema`.
pub(crate) fn get_schema() -> String {
    info!("Generating GraphQL schema");
    RootNode::new_with_scalar_value(Query, EmptyMutation::<()>::new(), EmptySubscription::<()>::new())
        .as_schema_language()
}

mod tests {

    #[tokio::test]
    async fn save_schema() {
        std::fs::write("schema.graphql", super::get_schema()).expect("Unable to write './schema.graphql' file");
    }

    #[tokio::test]
    async fn print_gql() {
        let config = super::Config::new();

        let gql_request = super::GraphQLRequest::<super::MyScalarValue> {
            query: r#"query { devsPerLanguage { aggregations {agg {buckets {key, docCount}}} }}"#.to_string(),
            operation_name: None,
            variables: None,
        };

        let _gql_data = super::execute_gql(&config, gql_request).await;
    }
}
