import { defineStore } from "pinia";

// Amount of experience required for a particular technology
export interface TechExperience {
  // Experience in Lines of Code.
  // 0 - any
  // 1 - average
  // 2 - used more than others
  loc: number;
  // Experience in years of use.
  // 0 - any
  // 1 - 10 number of years
  years: number;
}

export const useQueryStore = defineStore({
  id: "query",
  state: () => ({
    // Experience (value) per tech (key).
    tech: new Map<string, TechExperience>(),
    techListLoaded: false,
  }),
});