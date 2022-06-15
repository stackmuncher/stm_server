use crate::config::Config;
use juniper::http::{GraphQLRequest, GraphQLResponse};
use juniper::{EmptyMutation, EmptySubscription, RootNode};
use stackmuncher_lib::graphql::RustScalarValue;
use tracing::{error, info};

// a list of query resolvers
mod query;

/// A container for Query resolvers implemented in separate files.
struct Query;

/// A context structure for passing to query resolvers
struct Ctx {
    es_url: String,
    dev_idx: String,
}
impl juniper::Context for Ctx {}

/// Executes the provided GQL request, logs errors and returns a tuple with
/// 0. GraphQL response as String
/// 1. A flag indicating the success of the execution
pub(crate) async fn execute_gql(
    config: &Config,
    gql_request: GraphQLRequest<RustScalarValue>,
) -> Result<(String, Result<(), ()>), ()> {
    info!("Executing GQL request");

    // a struct to be passed to resolvers for accessing backend resources
    let context = Ctx {
        es_url: config.es_url.clone(),
        dev_idx: config.dev_idx.clone(),
    };

    // the GQL schema is static and can be reused between calls
    let root_node =
        RootNode::new_with_scalar_value(Query, EmptyMutation::<Ctx>::new(), EmptySubscription::<Ctx>::new());

    // execute the GQL query from `gql_request` using GQL resolvers
    let op = gql_request.operation_name.as_deref();
    let vars = &gql_request.variables();

    info!("GQL: {}", &gql_request.query[..gql_request.query.len().min(100)].replace("\n", " "));

    let res = juniper::execute(&gql_request.query, op, &root_node, vars, &context).await;

    // `result is to passed back to the caller as an indication if there were any errors in the execution
    let mut result = Ok(());

    // log any execution errors
    // they will be returned as part of the response to the caller inside GQL response, but we need to know about them at the back end
    if let Err(e) = &res {
        error!("GQL execution error: {}, gql: {:?}", e, gql_request);
        result = Err(());
    } else {
        for e in &res.as_ref().unwrap().1 {
            error!("GQL field error: {:?}, gql: {:?}", e, gql_request);
            result = Err(());
        }
    }

    // convert the response into GQL format of {"data": ..., "error": ...} for returning back to the caller over HTTP
    let res = GraphQLResponse::from_result(res);
    match serde_json::to_string(&res) {
        Ok(v) => Ok((v, result)),
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

#[tokio::test]
async fn save_schema() {
    std::fs::write("samples/schema.graphql", get_schema()).expect("Unable to write 'samples/schema.graphql' file");
}
