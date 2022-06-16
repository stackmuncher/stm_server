<script setup lang="ts">
import { devListForStack } from "@/graphql/queries";
import type { devListForStackVars } from "@/graphql/queries";
import { useQuery } from "@vue/apollo-composable";
import { computed } from "vue";
import { useQueryStore, PROFILES_PER_PAGE } from "@/stores/QueryStore";
import DevCard from "./DevCard.vue";
import MatchingDevsPaginationVue from "./MatchingDevsPagination.vue";

/** Adds local data to the stack from store to make a complete set of vars needed for the query. */
const useQueryVars = computed(() => {
  let retValue: devListForStackVars = {
    stack: [],
    pkgs: [],
    resultsFrom: store.currentPageProfiles * PROFILES_PER_PAGE,
  };

  Object.assign(retValue, store.stackVar);

  // console.log(retValue);

  return retValue;
});

const store = useQueryStore();

const { result, loading, error } = useQuery(
  devListForStack,
  useQueryVars,
  store.defaultApolloOptions
);
</script>

<template>
  <MatchingDevsPaginationVue v-if="!loading && result && !error" />
  <h6 v-if="loading">Loading ...</h6>

  <h2
    class="pe-md-5 text-muted"
    v-if="!loading && (!result || result.devListForStack.length == 0)"
  >
    Could not find anyone with these exact skills
  </h2>
  <div v-for="dev in result?.devListForStack" :key="dev.login">
    <DevCard :dev-details="dev" />
  </div>

  <p v-if="error" class="text-danger">
    <small>{{ error }}</small>
  </p>
  <MatchingDevsPaginationVue v-if="!loading && result && !error" />
</template>
