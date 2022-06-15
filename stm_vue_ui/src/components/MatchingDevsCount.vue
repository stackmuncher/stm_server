<script setup lang="ts">
import { devCountForStack } from "@/graphql/queries";
import { useQuery } from "@vue/apollo-composable";
import { ref, watch } from "vue";
import { useQueryStore } from "@/stores/QueryStore";
import { numFmt } from "@/formatters";
import { debounce } from "throttle-debounce";

const store = useQueryStore();

/** The number of matched profiles */
const count = ref(0);

/** Refetches GQL data via useQuery.
 * Needed because Apollo's built-in debounce and throttle are not working.
 */
const debounceRefetch = debounce(2000, () => {
  // console.log("dev-count debounceRefetch");
  refetch(store.stackVar);
});

/** Removes a pkg from the search */
const removePkg = (t: string) => {
  store.pkg.delete(t);
};

/** Removes a tech from the search */
const removeTech = (t: string) => {
  store.tech.delete(t);
};

const { result, loading, error, refetch } = useQuery(
  devCountForStack,
  store.stackVar,
  store.defaultApolloOptions
);

// update the dev count from the search results
watch(result, (value) => {
  // console.log(value);
  if (value) {
    count.value = value.devCountForStack;
  }
});

watch(store.tech, () => {
  // console.log(`watch for store.tech: ${value} `);
  // tell the template we are waiting for more input
  count.value = -1;
  debounceRefetch();
});

watch(store.pkg, () => {
  // console.log(`watch for store.pkg: ${value} `);
  // tell the template we are waiting for more input
  count.value = -1;
  debounceRefetch();
});

// for debugging
// watch(store.tech, async (tNew, tOld) => {
//   console.log("Watch tech (new/old)");
//   console.log(tNew);
//   console.log(tOld);
// });
</script>

<template>
  <h6>
    <span v-if="store.stackVar.stack.length == 0"> Total profiles: </span>
    <span v-else> Matching profiles: </span>

    <span v-if="loading"> counting ...</span>
    <span v-else-if="!loading && count < 0"> waiting ...</span>
    <span v-else>{{ numFmt(count) }}</span>
  </h6>
  <ul class="text-muted list-inline">
    <li class="list-inline-item">Stack:</li>
    <li v-if="store.isEmptyStack" class="list-inline-item">any</li>

    <li
      v-for="t in store.stackVar.stack"
      :key="t.tech"
      class="me-3 mb-3 bg-light text-dark rounded border text-wrap p-1 list-inline-item"
    >
      {{ t.tech }}
      <span
        @click="() => removeTech(t.tech)"
        class="badge bg-secondary p-1 ms-2"
        style="cursor: pointer"
        title="Remove from the filter"
      >
        x
      </span>
    </li>
    <li
      v-for="pkg in store.stackVar.pkgs"
      :key="pkg"
      class="me-3 mb-3 bg-light text-dark rounded border text-wrap p-1 list-inline-item"
    >
      {{ pkg }}
      <span
        @click="() => removePkg(pkg)"
        class="badge bg-secondary p-1 ms-2"
        style="cursor: pointer"
        title="Remove from the filter"
      >
        x
      </span>
    </li>
  </ul>
  <p v-if="error" class="text-danger">
    <small>{{ error }}</small>
  </p>
</template>
