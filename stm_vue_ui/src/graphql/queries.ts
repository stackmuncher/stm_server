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

export const devListForStack = gql`
  query ($stack: [TechExperience!]!, $pkgs: [String!]!) {
    devListForStack(stack: $stack, pkgs: $pkgs) {
      login
      name
      company
      blog
      location
      bio
      createdAt
      updatedAt
      description
      report {
        timestamp
        lastContributorCommitDateIso
        firstContributorCommitDateIso
        dateInit
        dateHead
        listCounts {
          tech
          contributorGitIds
          perFileTech
          fileTypes
          reportsIncluded
          projectsIncluded
          gitIdsIncluded
          contributors
          treeFiles
          recentProjectCommits
          keywords
        }
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
        }
        fileTypes {
          k
          c
        }
        projectsIncluded {
          projectName
          githubUserName
          githubRepoName
          contributorFirstCommit
          contributorLastCommit
          loc
          libs
          locProject
          libsProject
          ppl
          commitCount
          commitCountProject
          tech {
            language
            loc
            libs
            locPercentage
          }
        }
        commitTimeHisto {
          histogramRecentSum
          histogramAllSum
          histogramRecentStd
          histogramAllStd
          histogramRecent {
            h00
            h01
            h02
            h03
            h04
            h05
            h06
            h07
            h08
            h09
            h10
            h11
            h12
            h13
            h14
            h15
            h16
            h20
            h21
            h22
            h23
          }
          histogramAll {
            h00
            h01
            h02
            h03
            h04
            h05
            h06
            h07
            h08
            h09
            h10
            h11
            h12
            h13
            h14
            h15
            h16
            h20
            h21
            h22
            h23
          }
          timezoneOverlapRecent {
            h00
            h01
            h02
            h03
            h04
            h05
            h06
            h07
            h08
            h09
            h10
            h11
            h12
            h13
            h14
            h15
            h16
            h20
            h21
            h22
            h23
          }
          timezoneOverlapAll {
            h00
            h01
            h02
            h03
            h04
            h05
            h06
            h07
            h08
            h09
            h10
            h11
            h12
            h13
            h14
            h15
            h16
            h20
            h21
            h22
            h23
          }
        }
      }
    }
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
