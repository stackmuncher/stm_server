import gql from "graphql-tag";

// Corresponds to inpTechExperience input block
export interface inpTechExperience {
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

/** Manually created and incomplete type for report/tech struct. */
export interface Tech {
  language: string;
  files: number;
  codeLines: number;
  history?: {
    months: number;
    fromDateIso: string;
    toDateIso: string;
  };
  refs?: {
    k: string;
    c: number;
  }[];
  pkgs?: {
    k: string;
    c: number;
  }[];
}

/** Manually created and incomplete type for report/projects_included struct. */
export interface IncludedProject {
  projectName: string;
}

/** Manually created and incomplete type for dev struct. */
export interface DevListForStack {
  login?: string;
  name?: string;
  email?: string;
  blog?: string;
  location?: string;
  ownerId: string;
  report?: {
    timestamp: string;
    lastContributorCommitDateIso?: string;
    firstContributorCommitDateIso?: string;
    dateInit?: string;
    dateHead?: string;

    tech?: Tech[];
    projectsIncluded?: IncludedProject[];
  };
}

export interface devListForStackVars {
  stack: inpTechExperience[];
  pkgs: string[];
  resultsFrom: number;
}

export const devListForStack = gql`
  query ($stack: [TechExperience!]!, $pkgs: [String!]!, $resultsFrom: Int!) {
    devListForStack(stack: $stack, pkgs: $pkgs, resultsFrom: $resultsFrom) {
      login
      name
      email
      company
      blog
      location
      bio
      createdAt
      updatedAt
      description
      publicRepos
      ownerId
      report {
        timestamp
        lastContributorCommitDateIso
        firstContributorCommitDateIso
        dateInit
        dateHead
        tech {
          language
          files
          codeLines
          history {
            months
            fromDateIso
            toDateIso
          }
          refs {
            k
            c
          }
          pkgs {
            k
            c
          }
        }
        fileTypes {
          k
          c
        }
        projectsIncluded {
          projectName
        }
      }
    }
  }
`;

// export const devListForStack = gql`
//   query ($stack: [TechExperience!]!, $pkgs: [String!]!) {
//     devListForStack(stack: $stack, pkgs: $pkgs) {
//       login
//       name
//       email
//       company
//       blog
//       location
//       bio
//       createdAt
//       updatedAt
//       description
//       publicRepos
//       ownerId
//       report {
//         timestamp
//         lastContributorCommitDateIso
//         firstContributorCommitDateIso
//         dateInit
//         dateHead
//         listCounts {
//           tech
//           contributorGitIds
//           perFileTech
//           fileTypes
//           reportsIncluded
//           projectsIncluded
//           gitIdsIncluded
//           contributors
//           treeFiles
//           recentProjectCommits
//           keywords
//         }
//         tech {
//           language
//           files
//           codeLines
//           history {
//             months
//             fromDateIso
//             toDateIso
//           }
//           refs {
//             k
//             c
//           }
//           pkgs {
//             k
//             c
//           }
//         }
//         fileTypes {
//           k
//           c
//         }
//         projectsIncluded {
//           projectName
//           githubUserName
//           githubRepoName
//           contributorFirstCommit
//           contributorLastCommit
//           loc
//           libs
//           locProject
//           libsProject
//           ppl
//           commitCount
//           commitCountProject
//           tech {
//             language
//             loc
//             libs
//             locPercentage
//           }
//         }
//         commitTimeHisto {
//           histogramRecentSum
//           histogramAllSum
//           histogramRecentStd
//           histogramAllStd
//           histogramRecent {
//             h00
//             h01
//             h02
//             h03
//             h04
//             h05
//             h06
//             h07
//             h08
//             h09
//             h10
//             h11
//             h12
//             h13
//             h14
//             h15
//             h16
//             h20
//             h21
//             h22
//             h23
//           }
//           histogramAll {
//             h00
//             h01
//             h02
//             h03
//             h04
//             h05
//             h06
//             h07
//             h08
//             h09
//             h10
//             h11
//             h12
//             h13
//             h14
//             h15
//             h16
//             h20
//             h21
//             h22
//             h23
//           }
//           timezoneOverlapRecent {
//             h00
//             h01
//             h02
//             h03
//             h04
//             h05
//             h06
//             h07
//             h08
//             h09
//             h10
//             h11
//             h12
//             h13
//             h14
//             h15
//             h16
//             h20
//             h21
//             h22
//             h23
//           }
//           timezoneOverlapAll {
//             h00
//             h01
//             h02
//             h03
//             h04
//             h05
//             h06
//             h07
//             h08
//             h09
//             h10
//             h11
//             h12
//             h13
//             h14
//             h15
//             h16
//             h20
//             h21
//             h22
//             h23
//           }
//         }
//       }
//     }
//   }
// `;

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

/** Query variables for keywordSuggesterQuery */
export interface keywordSuggesterQueryVars {
  startsWith: string;
}
