<script setup lang="ts">
import { devsPerLanguageQuery } from "@/graphql/queries";
import { useQuery } from "@vue/apollo-composable";

const { result } = useQuery(devsPerLanguageQuery);

// watch(result, (value) => {
//   console.log(value);
// });
</script>

<template>
  <h6 class="mt-5 mb-3">Developers per language</h6>
  <ul v-if="result && result.devsPerLanguage" class="list-inline">
    <li
      v-for="bucket in result.devsPerLanguage.aggregations.agg.buckets"
      :key="bucket.key"
      class="list-inline-item bg-light text-dark p-1 rounded mb-3 border me-4"
    >
      <a
        :title="`${bucket.key} developers`"
        style="text-decoration: underline #6c757d"
        class="text-dark"
        href="/?{{ bucket.key }}"
      >
        {{ bucket.key }}
        <span class="badge bg-white text-dark ms-2" style="font-weight: 300">{{
          bucket.docCount
        }}</span></a
      >
    </li>
  </ul>
</template>
