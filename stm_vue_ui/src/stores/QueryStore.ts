import { defineStore } from "pinia";

export const useQueryStore = defineStore({
  id: "query",
  state: () => ({
    tech: new Array<string>(),
    // Experience in lines of code (tech name / band of experience).
    // The band of exp is relative to how much code is written in that language on average.
    // E.g. Java - lots, Docker - very little
    expLoc: new Map<string, number>(),
    // Experience in years (tech name / years in use)
    expYears: new Map<string, number>(),
  }),
  // getters: {
  //   doubleCount: (state) => state.counter * 2,
  // },
  // actions: {
  //   increment() {
  //     this.counter++;
  //   },
  // },
});
