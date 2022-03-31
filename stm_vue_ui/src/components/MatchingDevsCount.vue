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

const stackVar = computed(() => {
  const stack = new Array<inpTechExperienceInterface>();

  tech.value.forEach((v, k) => {
    stack.push({ tech: k, locBand: v.loc } as inpTechExperienceInterface);
  });

  console.log("stackVar computed");
  console.log(stack);

  let x = {
    stack: stack,
  };

  return x;
});

const { result, loading, error } = useQuery(devCountForStack, stackVar);

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
});
</script>

<template>
  <h6>
    <span v-if="stackVar.stack.length == 0"> Total profiles: </span>
    <span v-else> Matching profiles: </span>

    <span v-if="loading"> Loading ...</span>
    <span v-else>{{ count }}</span>
  </h6>
  <ul class="text-muted list-inline">
    <li v-for="t in stackVar.stack" :key="t.tech" class="list-inline-item">
      {{ t.tech }}
    </li>
  </ul>
  <p v-if="error" class="text-danger">
    <small>{{ error }}</small>
  </p>
</template>
