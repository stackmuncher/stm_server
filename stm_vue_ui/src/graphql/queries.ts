import gql from "graphql-tag";

// Corresponds to inpTechExperience input block
export interface inpTechExperienceInterface {
  tech: string;
  locBand: number;
}

export const inpTechExperience = gql`
  input TechExperience {
    tech: String!
    locBand: Int
  }
`;

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

export const devCountForStack = gql`
  query ($stack: [TechExperience!]!, $pkgs: [String!]!) {
    devCountForStack(stack: $stack, pkgs: $pkgs)
  }
`;

export const keywordSuggesterQuery = gql`
  query ($startsWith: String!) {
    keywordSuggester(startsWith: $startsWith) {
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
