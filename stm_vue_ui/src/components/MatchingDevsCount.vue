<script setup lang="ts">
import { devCountForStack } from "@/graphql/queries";
import type { inpTechExperienceInterface } from "@/graphql/queries";
import { useQuery } from "@vue/apollo-composable";
import { ref, watch } from "vue";
import { useQueryStore } from "@/stores/QueryStore";
import { computed } from "@vue/reactivity";

const store = useQueryStore();

const tech = ref(store.tech);

const count = ref(0);

const stackVar = () => {
  const stack = new Array<inpTechExperienceInterface>();

  tech.value.forEach((v, k) => {
    stack.push({ tech: k, locBand: v.loc } as inpTechExperienceInterface);
  });

  console.log("stackVar() called");
  console.log(stack);

  let x = {
    stack: stack,
  };

  return x;
};

// const hasData = computed(() => {
//   console.log(`Computed: ${tech.value.size}`);
//   return tech.value.size > 0;
// });

// // const x = ref(false);

// console.log(`Setup hasData: ${hasData.value}`);

const { result, loading, error, refetch } = useQuery(
  devCountForStack,
  stackVar()
);

watch(result, (value) => {
  console.log(value);
  if (value) {
    count.value = value.devCountForStack;
  }
});

watch(store.tech, async (tNew, tOld) => {
  console.log("Watch tech (new/old)");
  console.log(tNew);
  console.log(tOld);

  if (tNew.size > 0) {
    console.log("refetching");
    await refetch(stackVar());
  }
});

// watch(hasData, (newHasData, oldHasData) => {
//   console.log(`Watch hasData: ${oldHasData} -> ${newHasData}`);
// });
</script>

<template>
  <p>{{ tech }} / has data: {{ tech.size > 0 }}</p>
  <p v-if="error">{{ error }}</p>
  <h6>
    Matches: <span v-if="loading"> Loading ...</span
    ><span v-else>{{ count }}</span>
  </h6>
</template>
