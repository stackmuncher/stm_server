<script setup lang="ts">
import { devListForStack } from "@/graphql/queries";
import { useQuery } from "@vue/apollo-composable";
import { useQueryStore } from "@/stores/QueryStore";

const store = useQueryStore();

const { result, loading, error } = useQuery(devListForStack, store.stackVar);
</script>

<template>
  <h6>
    <span v-if="loading"> Loading ...</span>
    <span v-else>List of Devs</span>
  </h6>
  <ul class="text-muted list-inline">
    <li
      v-for="dev in result?.devListForStack"
      :key="dev.login"
      class="me-3 mb-3 bg-light text-dark rounded border text-wrap p-1 list-inline-item"
    >
      {{ dev.login }}
    </li>
  </ul>
  <p v-if="error" class="text-danger">
    <small>{{ error }}</small>
  </p>
</template>
