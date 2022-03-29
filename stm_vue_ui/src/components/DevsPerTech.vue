<script setup lang="ts">
import { devsPerLanguageQuery } from "@/graphql/queries";
import { useQuery } from "@vue/apollo-composable";
import { ref } from "vue";
// import type { Ref } from "vue";
import { useQueryStore } from "@/stores/QueryStore";

const store = useQueryStore();

const tech = ref(store.tech);

// Experience in lines of code (tech name / band of experience).
// The band of exp is relative to how much code is written in that language on average.
// E.g. Java - lots, Docker - very little
// const expLoc: Ref<Map<string, number>> = ref(new Map<string, number>());
const expLoc = ref(store.expLoc);

const anyLoc = "any LoC";
const anyYears = "any duration";

// Experience in years (tech name / years in use)
// const expYears: Ref<Map<string, number>> = ref(new Map<string, number>());
const expYears = ref(store.expYears);

// A flag to toggle visibility of tech card details. It should be computed()
const selected_tech_class = (t: string) =>
  tech.value.includes(t) ? "visible" : "invisible";

// Sets expLoc for a particular tech (t) in expLoc or removes it if `any`. It should be computed()
const setLoCParam = (t: string, v: string) => {
  console.log(`Setting expLoc ${t} / ${v}`);

  switch (v) {
    case anyLoc:
      expLoc.value.delete(t);
      break;
    case "some":
      expLoc.value.set(t, 1);
      break;
    case "a lot":
      expYears.value.set(t, 2);
      break;
    default:
      console.error(`Unknown expLoc option ${v}. It's a bug.`);
  }
};

// Gets expLoc for a particular tech (t) in expLoc or `any`. It should be computed()
const getLoCParam = (t: string) => {
  // console.log(`Getting expLoc ${t}`);
  const v = expLoc.value.get(t);
  if (v) {
    switch (v) {
      case 1:
        return "some";
      case 2:
        return "a lot";
      default:
        console.error(`Unknown expLoc option ${v}. It's a bug.`);
    }
  } else {
    return anyLoc;
  }
};

// Sets expYears for a particular tech (t) in expYears or removes it if `any`. It should be computed()
const setYearsParam = (t: string, v: string) => {
  console.log(`Setting expYears ${t} / ${v}`);

  switch (v) {
    case anyYears:
      expYears.value.delete(t);
      break;
    case "1 year":
      expYears.value.set(t, 1);
      break;
    case "2 years":
      expYears.value.set(t, 2);
      break;
    case "3 years":
      expYears.value.set(t, 3);
      break;
    case "5 years":
      expYears.value.set(t, 5);
      break;
    case "10 years":
      expYears.value.set(t, 10);
      break;
    default:
      console.error(`Unknown expYears option ${v}. It's a bug.`);
  }
};

const getYearsParam = (t: string) => {
  // console.log(`Getting expYears ${t}`);
  const v = expYears.value.get(t);
  if (v) {
    switch (v) {
      case 1:
        return "1 year";
      case 2:
        return "2 years";
      case 3:
        return "3 years";
      case 5:
        return "5 years";
      case 10:
        return "10 years";
      default:
        console.error(`Unknown expYears option ${v}. It's a bug.`);
        return anyYears;
    }
  } else {
    return anyYears;
  }
};

const { result } = useQuery(devsPerLanguageQuery);

// watch(result, (value) => {
//   console.log(value);
// });
</script>

<template>
  <h6 class="mt-5 mb-3">Developers per language</h6>
  <p>{{ tech }} / {{ expLoc }} / {{ expYears }}</p>
  <div v-if="result && result.devsPerLanguage" class="row g-3">
    <div
      v-for="bucket in result.devsPerLanguage.aggregations.agg.buckets"
      :key="bucket.key"
      class="col-12 col-md-6 col-xl-4"
    >
      <div class="bg-light text-dark rounded border p-2">
        <input
          type="checkbox"
          :id="bucket.key"
          :value="bucket.key"
          v-model="tech"
          class="me-2"
        />
        <label
          :for="bucket.key"
          :title="`${bucket.key} developers`"
          class="text-dark fs-5"
        >
          {{ bucket.key }}
          <div class="badge bg-white text-dark ms-2" style="font-weight: 300">
            <span class="team-badge"> {{ bucket.docCount }}</span>
          </div></label
        >
        <div :class="selected_tech_class(bucket.key)" class="row mt-2 gx-3">
          <div class="col-auto" title="Minimum experience in Lines of Code">
            <span class="loc-badge"
              ><select
                :value="getLoCParam(bucket.key)"
                @change="(event) => setLoCParam(bucket.key, (event.target as HTMLSelectElement).value)"
              >
                <option>{{ anyLoc }}</option>
                <option>some</option>
                <option>a lot</option>
              </select></span
            >
          </div>
          <div class="col-auto" title="Minimum experience in years">
            <span class="calendar-badge ms-2"
              ><select
                :value="getYearsParam(bucket.key)"
                @change="(event) => setYearsParam(bucket.key, (event.target as HTMLSelectElement).value)"
              >
                <option>{{ anyYears }}</option>
                <option>1 year</option>
                <option>2 years</option>
                <option>3 years</option>
                <option>5 years</option>
                <option>10 years</option>
              </select></span
            >
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
