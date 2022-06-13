<script setup lang="ts">
import { devListForStack } from "@/graphql/queries";
import { useQuery } from "@vue/apollo-composable";
import { useQueryStore } from "@/stores/QueryStore";
import DevCard from "./DevCard.vue";

const store = useQueryStore();

const { result, loading, error } = useQuery(
  devListForStack,
  store.stackVar,
  store.defaultApolloOptions
);
</script>

<template>
  <h6>
    <span v-if="loading"> Loading ...</span>
    <span v-else>List of Devs</span>
  </h6>

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
</template>
