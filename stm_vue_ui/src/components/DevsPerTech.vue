<script setup lang="ts">
import { devsPerLanguageQuery } from "@/graphql/queries";
import { useQuery } from "@vue/apollo-composable";
import { ref, watch } from "vue";
import { useQueryStore } from "@/stores/QueryStore";
import { numFmt } from "@/formatters";

const store = useQueryStore();

const tech = ref(store.tech);

const anyLoc = "any LoC";
const anyYears = "any duration";

// A flag to toggle visibility of tech card details. It should be computed()
const selected_tech_class = (t: string) =>
  tech.value.has(t) ? "visible" : "invisible";

const toggleTechExp = (t: string, v: boolean) => {
  if (!v) {
    tech.value.delete(t);
  } else if (!tech.value.has(t)) {
    // create a new tech exp with defaults
    tech.value.set(t, { loc: 0, years: 0 });
  }
};

// Sets expLoc for a particular tech (t) in expLoc or removes it if `any`. It should be computed()
const setTechExp = (t: string, loc: string | null, years: string | null) => {
  // console.log(`Setting TechExp ${t} / loc:${loc}, years:${years}`);

  const techExp = tech.value.get(t);

  if (!techExp) {
    console.error(
      `Tech record for ${t} not found in local Vue store. It's a bug.`
    );
    return;
  }

  switch (loc) {
    case null:
      break;
    case anyLoc:
      techExp.loc = 0;
      break;
    case "some":
      techExp.loc = 1;
      break;
    case "a lot":
      techExp.loc = 2;
      break;
    default:
      console.error(`Unknown expLoC option ${loc}. It's a bug.`);
  }

  switch (years) {
    case null:
      break;
    case anyYears:
      techExp.years = 0;
      break;
    case "1 year":
      techExp.years = 1;
      break;
    case "2 years":
      techExp.years = 2;
      break;
    case "3 years":
      techExp.years = 3;
      break;
    case "5 years":
      techExp.years = 5;
      break;
    case "10 years":
      techExp.years = 10;
      break;
    default:
      console.error(`Unknown expYears option ${years}. It's a bug.`);
  }
};

// Gets expLoc for a particular tech (t) in expLoc or `any`. It should be computed()
const getLoCParam = (t: string) => {
  // console.log(`Getting expLoc ${t}`);
  const v = tech.value.get(t);

  if (v) {
    switch (v.loc) {
      case 0:
        return anyLoc;
      case 1:
        return "some";
      case 2:
        return "a lot";
      default:
        console.error(`Unknown loc value: ${v.loc}. It's a bug.`);
        return anyLoc;
    }
  } else {
    return anyLoc;
  }
};

const getYearsParam = (t: string) => {
  // console.log(`Getting expYears ${t}`);
  const v = tech.value.get(t);
  if (v) {
    switch (v.years) {
      case 0:
        return anyYears;
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
        console.error(`Unknown expYears option ${v.years}. It's a bug.`);
        return anyYears;
    }
  } else {
    return anyYears;
  }
};

const { result, loading } = useQuery(devsPerLanguageQuery);

watch(result, (value) => {
  // console.log("devsPerLanguageQuery result arrived");
  // console.log(value);
  if (value) {
    store.techListLoaded = true;
  }
});
</script>

<template>
  <h6 v-if="loading">Loading popular stacks ...</h6>
  <div v-if="result && result.devsPerLanguage" class="row g-3">
    <div
      v-for="bucket in result.devsPerLanguage.aggregations.agg.buckets"
      v-show="
        store.searchFilter.length == 0 ||
        tech.has(bucket.key) ||
        bucket.key.startsWith(store.searchFilter)
      "
      :key="bucket.key"
      class="col-12 col-md-6 col-xl-4"
    >
      <div class="bg-light text-dark rounded border p-2">
        <input
          type="checkbox"
          :checked="store.tech.has(bucket.key)"
          :id="bucket.key"
          :value="bucket.key"
          @change="(event) => toggleTechExp(bucket.key, (event.target as HTMLInputElement).checked)"
          class="me-2"
        />
        <label
          :for="bucket.key"
          :title="`${bucket.key} developers`"
          class="text-dark fs-5"
        >
          {{ bucket.key }}
          <div class="badge bg-white text-dark ms-2" style="font-weight: 300">
            <span class="team-badge"> {{ numFmt(bucket.docCount) }}</span>
          </div></label
        >
        <div :class="selected_tech_class(bucket.key)" class="row mt-2 gx-3">
          <div class="col-auto" title="Minimum experience in Lines of Code">
            <span class="loc-badge"
              ><select
                :value="getLoCParam(bucket.key)"
                @change="(event) => setTechExp(bucket.key, (event.target as HTMLSelectElement).value, null)"
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
                @change="(event) => setTechExp(bucket.key, null, (event.target as HTMLSelectElement).value)"
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
