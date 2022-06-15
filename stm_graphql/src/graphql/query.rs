use super::{Ctx, Query};
use crate::elastic;
use juniper::{graphql_object, FieldResult};
use stackmuncher_lib::graphql::RustScalarValue;
use stm_shared::elastic as elastic_shared;
use stm_shared::elastic::types as es_types;

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
        pkgs: Vec<String>,
    ) -> FieldResult<i32> {
        // get number of devs per technology
        let dev_count = match elastic::matching_dev_count(&context.es_url, &context.dev_idx, stack, pkgs, 0, 0).await {
            Ok(v) => v,
            Err(_) => return Err("ES query failed. See server logs.".into()),
        };

        Ok(dev_count)
    }

    /// Returns the list of devs matching the stack starting from `results_from`.
    /// Negative `results_from` is set to 0.  
    /// Only `MAX_DEV_LISTINGS_PER_SEARCH_RESULT` results are returned.
    /// See stm_graphql/samples/gql-responses/devListForStack.gql.json for the full example.
    async fn dev_list_for_stack<'db>(
        &self,
        context: &'db Ctx,
        stack: Vec<elastic::TechExperience>,
        pkgs: Vec<String>,
        results_from: i32,
    ) -> FieldResult<Vec<es_types::GitHubUser>> {
        // get number of devs per technology
        let dev_list = match elastic::dev_list_for_stack(
            &context.es_url,
            &context.dev_idx,
            stack,
            pkgs,
            0,
            0,
            results_from.max(0) as u32,
            elastic::EsSortType::RecentlyActive,
            elastic::EsSortDirection::Desc,
        )
        .await
        {
            Ok(v) => v,
            Err(_) => return Err("ES query failed. See server logs.".into()),
        };

        Ok(dev_list)
    }

    /// Returns a list of keywords starting with what the user typed in so far.
    /// See stm_graphql/samples/es-responses/devs_per_language.json for the full example.
    async fn keyword_suggester<'db>(
        &self,
        context: &'db Ctx,
        starts_with: String,
    ) -> FieldResult<Option<elastic_shared::types::ESAggs>> {
        // get number of devs per technology
        let keywords = match elastic::keyword_suggester(&context.es_url, &context.dev_idx, starts_with).await {
            Ok(v) => v,
            Err(_) => return Err("ES query failed. See server logs.".into()),
        };

        Ok(keywords)
    }
}

// IMPORTANT: GQL errors are logged in the sample response output files
// remember to check them if the tests fail
#[tokio::test]
async fn devs_per_language_test() {
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
async fn dev_count_for_stack_test() {
    let config = super::Config::new();

    let gql_request = super::GraphQLRequest::<super::RustScalarValue> {
        query: r#"query { devCountForStack (stack: [{tech: "rust"}], pkgs: ["serde"])}"#.to_string(),
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

#[tokio::test]
async fn dev_list_for_stack_test() {
    let config = super::Config::new();

    let gql_request = super::GraphQLRequest::<super::RustScalarValue> {
        query: r#"query { devListForStack (stack: [{tech: "rust"}], pkgs: ["serde"], resultsFrom: 0) 
        { 
            login, name, company, blog, location, bio, createdAt, updatedAt, description, publicRepos, ownerId,
            report {
                timestamp, lastContributorCommitDateIso, firstContributorCommitDateIso, dateInit, dateHead,
                listCounts { tech, contributorGitIds, perFileTech,fileTypes, reportsIncluded, projectsIncluded, gitIdsIncluded, contributors,treeFiles, recentProjectCommits, keywords }
                tech {
                    language, files, codeLines,
                    history { months, fromDateIso, toDateIso }
                    refs {k, c}
                }
                fileTypes {k, c}
                projectsIncluded {
                    projectName, githubUserName, githubRepoName, contributorFirstCommit, contributorLastCommit, loc, libs, locProject, libsProject, ppl, commitCount, commitCountProject,
                    tech { language, loc, libs, locPercentage }
                
                }
                commitTimeHisto {
                    histogramRecentSum, histogramAllSum, histogramRecentStd, histogramAllStd, 
                    histogramRecent { h00, h01, h02, h03, h04, h05, h06, h07, h08, h09, h10, h11, h12, h13, h14, h15, h16, h20, h21, h22, h23 }
                    histogramAll { h00, h01, h02, h03, h04, h05, h06, h07, h08, h09, h10, h11, h12, h13, h14, h15, h16, h20, h21, h22, h23 }
                    timezoneOverlapRecent { h00, h01, h02, h03, h04, h05, h06, h07, h08, h09, h10, h11, h12, h13, h14, h15, h16, h20, h21, h22, h23 }
                    timezoneOverlapAll{ h00, h01, h02, h03, h04, h05, h06, h07, h08, h09, h10, h11, h12, h13, h14, h15, h16, h20, h21, h22, h23 }
                }
            } 
        }
    }"#
        .to_string(),
        operation_name: None,
        variables: None,
    };

    let (gql_data, result) = super::execute_gql(&config, gql_request).await.unwrap();

    std::fs::write("samples/gql-responses/devListForStack.gql.json", gql_data.clone())
        .expect("Unable to write 'samples/gql-responses/devListForStack.gql.json' file");

    assert!(result.is_ok(), "devListForStack query executed with errors");
    assert!(
        gql_data.starts_with(r#"{"data":{"devListForStack":[{"login":""#),
        "Unexpected devListForStack query response"
    );
}

#[tokio::test]
async fn keyword_suggester_test() {
    let config = super::Config::new();

    let gql_request = super::GraphQLRequest::<super::RustScalarValue> {
        query: r#"query { keywordSuggester (startsWith: "mongo") { aggregations {agg {buckets {key, docCount}}}} }"#
            .to_string(),
        operation_name: None,
        variables: None,
    };

    let (gql_data, result) = super::execute_gql(&config, gql_request).await.unwrap();

    std::fs::write("samples/gql-responses/keywordSuggester.gql.json", gql_data.clone())
        .expect("Unable to write 'samples/gql-responses/keywordSuggester.gql.json' file");

    assert!(result.is_ok(), "keywordSuggester query executed with errors");
    assert!(gql_data.contains("mongodb"), "Unexpected devCountForStack query response");
}
