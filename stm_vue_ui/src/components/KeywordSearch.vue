<script setup lang="ts">
import { keywordSuggesterQuery } from "@/graphql/queries";
import { useQuery } from "@vue/apollo-composable";
import { ref, watch, computed } from "vue";
import { useQueryStore } from "@/stores/QueryStore";

const store = useQueryStore();

const pkg = ref(store.pkg);

const userInput = ref("");

const startsWithVar = computed(() => {
  console.log(`startsWithVar computed: ${userInput.value}`);

  let x = {
    startsWith: userInput.value,
  };

  return x;
});

const keywordSuggesterEnabled = computed(() => {
  return userInput.value.length > 3;
});

const { result, loading } = useQuery(
  keywordSuggesterQuery,
  startsWithVar,
  () => ({
    enabled: keywordSuggesterEnabled.value,
    debounce: 500,
  })
);

const togglePkg = (t: string, v: boolean) => {
  if (!v) {
    pkg.value.delete(t);
  } else if (!pkg.value.has(t)) {
    // create a new tech exp with defaults
    pkg.value.add(t);
  }
};

watch(result, (value) => {
  console.log("keywordSuggesterQuery result arrived");
  console.log(value);
});
</script>

<template>
  <div class="d-flex mt-4">
    <input
      class="form-control me-2"
      type="search"
      title="Start typing keywords of the required technology stack, e.g. C# + Twilio + Azure."
      v-model="userInput"
    />
  </div>
  <p v-if="!keywordSuggesterEnabled" class="mb-5 text-muted mt-2">
    Start typing keywords of the required technology stack, e.g.
    <code>typescript vuejs apollo</code> or <code>c# sql cosmos</code>
  </p>
  <p v-else class="mb-5 mt-2">
    <span v-if="loading"> Loading ...</span>
    <ul v-else class="list-inline">
      <li
        v-for="bucket in result.keywordSuggester.aggregations.agg.buckets"
        :key="bucket.key"
        class="me-3 mb-3 bg-light text-dark rounded border fs-5 text-wrap p-2 list-inline-item "
      >
       <input
          type="checkbox"
          :id="bucket.key"
          :value="bucket.key"
          @change="(event) => togglePkg(bucket.key, (event.target as HTMLInputElement).checked)"
          class="me-2"
        /> <label
          :for="bucket.key"
        >{{ bucket.key }} <div class="badge bg-white text-dark ms-2" style="font-weight: 300">
            <span class="team-badge"> {{ bucket.docCount }}</span>
          </div></label
        >
      </li>
    </ul>
  </p>
</template>
