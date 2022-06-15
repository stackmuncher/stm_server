<script setup lang="ts">
import { keywordSuggesterQuery } from "@/graphql/queries";
import type { keywordSuggesterQueryVars } from "@/graphql/queries";
import { useLazyQuery } from "@vue/apollo-composable";
import { ref, watch } from "vue";
import type { Ref } from "vue";
import { useQueryStore } from "@/stores/QueryStore";
import { storeToRefs } from "pinia";
import { numFmt } from "@/formatters";
import { debounce } from "throttle-debounce";

/** Keywords shorter than this value should not be sent to the server */
const MIN_KW_LENGTH = 3;

const store = useQueryStore();
const pkg = ref(store.pkg);
const { searchFilter } = storeToRefs(store);

// useLazyQuery requires reactive variables
// see https://github.com/vuejs/apollo/issues/1369
const queryVars: Ref<keywordSuggesterQueryVars> = ref({ startsWith: "" });

/** Re-fetches GQL data via useQuery.
 * Needed because Apollo's built-in debounce and throttle are not working. */
const debounceRefetch = debounce(2000, () => {
  // console.log(`kw-search debounceRefetch: ${searchFilter.value}`);
  // console.log(`queryVars.startsWith: ${queryVars.value.startsWith}`);
  const searchFilterTrimmed = searchFilter.value.trim().toLowerCase();
  // the search string must be long enough to send to the server
  if (searchFilterTrimmed.length > MIN_KW_LENGTH) {
    // and also it has to be different from the prev one - spaces or case changes should not affect the search
    if (queryVars.value.startsWith == searchFilterTrimmed) {
      // console.log("identical normalized search string");
      return;
    }
    queryVars.value.startsWith = searchFilterTrimmed;
    // console.log(`new queryVars.startsWith: ${queryVars.value.startsWith}`);
    // if a previous query is still loading then wait until it finishes
    // this should be removed for prod because it is detrimental to UX
    if (loading.value) {
      // console.log(`already loading: ${loading.value}`);
      setTimeout(() => {
        debounceRefetch();
      }, 1200);
    } else {
      // console.log("calling load() now");
      load();
    }
  }
});

const { result, loading, error, load } = useLazyQuery(
  keywordSuggesterQuery,
  queryVars,
  store.defaultApolloOptions
);

const togglePkg = (t: string, v: boolean) => {
  if (!v) {
    pkg.value.delete(t);
  } else if (!pkg.value.has(t)) {
    // create a new tech exp with defaults
    pkg.value.add(t);
  }
};

// triggers debouncing for GQL query based on the user input
watch(searchFilter, (value) => {
  // console.log(`watch for store.searchFilter: ${value} `);
  // ignore short input
  if (value.trim().length > MIN_KW_LENGTH) {
    // console.log("length > MIN_KW_LENGTH");
    debounceRefetch();
  } else {
    // remove previous results when the search is too short
    result.value = null;
  }
});

// for debugging only
// watch(result, (value) => {
//   console.log(`keywordSuggesterQuery result arrived: ${value}`);
//   console.log(value);
// });
</script>

<template>
  <div class="d-flex mt-4">
    <input
      class="form-control me-2"
      type="search"
      title="Start typing keywords of the required technology stack, e.g. C# + Twilio + Azure."
      v-model="store.searchFilter"
    />
  </div>
  <p v-if="loading" class="mb-5 text-muted mt-2">Loading ...</p>
  <p v-else-if="!result" class="mb-5 text-muted mt-2">
    Start typing keywords of the required technology stack, e.g.
    <code>typescript vuejs apollo</code> or <code>c# sql cosmos</code>
  </p>
  <div v-else class="mb-5 mt-2">
    <ul
      v-if="result?.keywordSuggester?.aggregations?.agg?.buckets"
      class="list-inline"
    >
      <li
        v-for="bucket in result.keywordSuggester.aggregations.agg.buckets"
        :key="bucket.key"
        class="me-3 mb-3 bg-light text-dark rounded border text-wrap p-1 list-inline-item"
      >
        <input
          type="checkbox"
          :checked="store.pkg.has(bucket.key)"
          :id="bucket.key"
          :value="bucket.key"
          @change="(event) => togglePkg(bucket.key, (event.target as HTMLInputElement).checked)"
          class="me-2"
        />
        <label :for="bucket.key">
          {{ bucket.key }}
          <div class="badge bg-white text-dark ms-2" style="font-weight: 300">
            <span class="team-badge"> {{ numFmt(bucket.docCount) }}</span>
          </div>
        </label>
      </li>
    </ul>
    <p v-else class="mb-5 text-muted mt-2">No matches found.</p>
  </div>
  <p v-if="error" class="text-danger">
    <small>{{ error }}</small>
  </p>
</template>
