<script setup lang="ts">
import { devCountForStack } from "@/graphql/queries";
import type { inpTechExperienceInterface } from "@/graphql/queries";
import { useQuery } from "@vue/apollo-composable";
import { ref, watch } from "vue";
import { useQueryStore } from "@/stores/QueryStore";
import { computed } from "@vue/reactivity";
import { numFmt } from "@/formatters";

const store = useQueryStore();

const tech = ref(store.tech);
const pkg = ref(store.pkg);

const count = ref(0);

const stackVar = computed(() => {
  const stack = new Array<inpTechExperienceInterface>();

  tech.value.forEach((v, k) => {
    stack.push({ tech: k, locBand: v.loc } as inpTechExperienceInterface);
  });

  let x = {
    stack: stack,
    pkgs: Array.from(pkg.value),
  };

  console.log("stackVar computed");
  console.log(x);

  return x;
});

const isEmptyStack = computed(
  () => stackVar.value.stack.length + stackVar.value.pkgs.length == 0
);

const removePkg = (t: string) => {
  pkg.value.delete(t);
};

const removeTech = (t: string) => {
  tech.value.delete(t);
};

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
    <span v-else>{{ numFmt(count) }}</span>
  </h6>
  <ul class="text-muted list-inline">
    <li class="list-inline-item">Stack:</li>
    <li v-if="isEmptyStack" class="list-inline-item">any</li>

    <li
      v-for="t in stackVar.stack"
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
      v-for="pkg in stackVar.pkgs"
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
