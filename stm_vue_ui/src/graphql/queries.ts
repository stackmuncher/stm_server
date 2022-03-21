import gql from "graphql-tag";

export const devsPerLanguageQuery = gql`
  query {
    devsPerLanguage {
      aggregations {
        agg {
          buckets {
            key
            docCount
          }
        }
      }
    }
  }
`;
