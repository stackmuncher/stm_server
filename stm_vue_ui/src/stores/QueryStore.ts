import { defineStore } from "pinia";
import type { inpTechExperienceInterface } from "@/graphql/queries";
import type { UseQueryOptions } from "@vue/apollo-composable";

/** Amount of experience required for a particular technology. */
export interface TechExperience {
  /** Experience in Lines of Code.
  0 - any
  1 - average
  2 - used more than others  */
  loc: number;

  /** Experience in years of use.
  0 - any
  1 - 10 number of years  */
  years: number;
}

/** The full list of top level tabs. */
export enum SearchTabNames {
  Search,
  Profiles,
  Shortlist,
  Message,
}

const defaultApolloOptions: UseQueryOptions = {
  fetchPolicy: "cache-first",
  notifyOnNetworkStatusChange: true,
};

/** A shared app store based on Pinia. */
export const useQueryStore = defineStore({
  id: "query",

  state: () => ({
    /** A list of Tech (key) and Experience (value) pairs for the target stack. */
    tech: new Map<string, TechExperience>(),

    /** List of packages selected for the target stack. */
    pkg: new Set<string>(),

    /** Is `true` when the list of main tech items is loaded into Apollo.
     * E.g. C#, C++, Rust
     */
    techListLoaded: false,

    /** A search string typed into the search box by the user. */
    searchFilter: "",

    /** Name of the currently active tab. Defaults to Search. */
    activeSearchTab: SearchTabNames.Search,
  }),

  getters: {
    /** Returns true if the target stack empty. */
    isEmptyStack: (state) => state.tech.size + state.pkg.size == 0,

    /** GQL variables for useQuery: converts the current search criteria to GQL format */
    stackVar: (state) => {
      const stack = new Array<inpTechExperienceInterface>();

      state.tech.forEach((v, k) => {
        stack.push({ tech: k, locBand: v.loc } as inpTechExperienceInterface);
      });

      const x = {
        stack: stack,
        pkgs: Array.from(state.pkg),
      };

      // console.log("stackVar computed");
      // console.log(x);

      return x;
    },

    /** The default Apollo options should be specified in the provider during the setup, but
     * I could not make it work. This is a quick workaround.
     */
    defaultApolloOptions: () => defaultApolloOptions,
  },
});
