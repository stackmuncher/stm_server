<script setup lang="ts">
import type { DevListForStack, Tech } from "@/graphql/queries";
import { useQueryStore } from "@/stores/QueryStore";
import { computed } from "vue";

const store = useQueryStore();

const props = defineProps<{
  /** Expects _source property from the ElasticSearch GQL response for this dev  */
  devDetails: DevListForStack;
}>();

/** The name is optional and may be completely absent. Built it out of what we have. */
const devName = computed(() => {
  if (!props.devDetails) return "devDetails not initialized yet";
  if (props.devDetails.name) return props.devDetails.name;
  if (props.devDetails.login) return props.devDetails.login;

  return "Anonymous Software Engineer";
});

/** DevID can either be a GH login or an internal STM owner ID. */
const devId = computed(() => {
  if (!props.devDetails) return "devDetails not initialized yet";

  if (props.devDetails.login) {
    return `/${props.devDetails.login}`;
  } else {
    return `/?dev=${props.devDetails.ownerId}`;
  }
});

/** Returns TRUE if public contact details are available */
const hasPublicContactDetails = computed(
  () =>
    props.devDetails &&
    props.devDetails.login &&
    (props.devDetails.blog || props.devDetails.email)
);

/** Only the year component of the current date. */
const yearNow = new Date().getFullYear();

/** Returns a phrase like `10 projects over 5 years` or an empty string if not enough data */
const projectsOverYears = computed(() => {
  // try to get the number of projects - is there enough data?
  if (!props.devDetails?.report?.projectsIncluded) return "";
  const projectCount = props.devDetails.report.projectsIncluded.length;
  if (projectCount == 0) return "";

  // get the year of the first commit
  // prefer first_contributor_commit_date_iso, then date_init, then now()
  // check if the data is present and is not in the future
  const firstContributorCommitDateIso = props.devDetails.report
    .firstContributorCommitDateIso
    ? Number.parseInt(
        props.devDetails.report.firstContributorCommitDateIso.substring(0, 4)
      )
    : 0;

  const dateInit = props.devDetails.report.dateInit
    ? Number.parseInt(props.devDetails.report.dateInit.substring(0, 4))
    : 0;

  const firstCommitYear =
    firstContributorCommitDateIso > 0 &&
    firstContributorCommitDateIso <= yearNow
      ? firstContributorCommitDateIso
      : dateInit > 0 && dateInit <= yearNow
      ? dateInit
      : yearNow;

  // calculate the total number of years of experience
  const years = yearNow - firstCommitYear + 1;

  // get plural or singular form
  const msgYearsPart = years > 1 ? "years" : "year";
  const msgProjectsPart = years > 1 ? "projects" : "project";

  // build the final output
  return `${projectCount} ${msgProjectsPart} over ${years} ${msgYearsPart}`;
});

/** Returns a list of languages that are in the dev's stack and the search filter */
const matchingLanguages = computed(() => {
  if (!props.devDetails?.report?.tech) return [];

  // get the list of languages from the search filter in an array form
  const listOfFilterLangs = Array.from(store.tech.keys()).map((key) =>
    key.toLowerCase()
  );

  // create an array of techs present in the filter
  const matchingLangs = props.devDetails.report.tech
    .map((tech) =>
      listOfFilterLangs.includes(tech.language?.toLowerCase()) ? tech : null
    )
    .filter((n) => n) as Tech[];

  // languages with most code lines come first
  matchingLangs.sort((a, b) => (b ? b.codeLines : 0) - (a ? a.codeLines : 0));

  return matchingLangs;
});

/** Returns a list of languages not in the list of search filter */
const otherLanguages = computed(() => {
  if (!props.devDetails?.report?.tech) return [];

  // get the list of languages from the search filter in an array form
  const listOfFilterLangs = Array.from(store.tech.keys()).map((key) =>
    key.toLowerCase()
  );

  // create an array of techs absent from the filter
  const otherLangs = props.devDetails.report.tech
    .map((tech) =>
      listOfFilterLangs.includes(tech.language?.toLowerCase()) ? null : tech
    )
    .filter((n) => n) as Tech[];

  // languages with most code lines come first
  otherLangs.sort((a, b) => (b ? b.codeLines : 0) - (a ? a.codeLines : 0));

  return otherLangs;
});

/** Returns a map of package names and their count. */
const matchingPkgs = computed(() => {
  // an output collector
  const matchingPkgs = new Map<string, number>();

  if (!props.devDetails?.report?.tech) return [];

  // get the list of languages from the search filter in an array form
  const listOfFilterPkgs = Array.from(store.pkg).map((value) =>
    value.toLowerCase()
  );

  // collect all dev packages present in the search filter and tot up their counts
  for (let tech of props.devDetails.report.tech) {
    if (tech.refs) {
      // pkgs per tech can be in 2 locations - refs and pkgs
      for (let pkg of tech.refs) {
        const k = pkg.k.toLowerCase();
        // match with the filter
        for (let kw of listOfFilterPkgs) {
          if (k.includes(kw)) {
            const c = matchingPkgs.get(kw);
            matchingPkgs.set(kw, c ? c + pkg.c : pkg.c);
          }
        }
      }
    }
    if (tech.pkgs) {
      for (let pkg of tech.pkgs) {
        const k = pkg.k.toLowerCase();
        // match with the filter
        for (let kw of listOfFilterPkgs) {
          if (k.includes(kw)) {
            const c = matchingPkgs.get(kw);
            matchingPkgs.set(kw, c ? c + pkg.c : pkg.c);
          }
        }
      }
    }
  }

  return Array.from(matchingPkgs, ([k, c]) => ({ k, c })).sort(
    (a, b) => b.c - a.c
  );
});

/** Formats number of months into years + months in 5 month increment, depending on how many years.
 * E.g. 0.5y, 2.5y, 3y or 5y.
 */
function months_to_years(months?: number) {
  const blankValue = "n/a";
  if (!months) return blankValue;

  // calculate the remainder of months ahead of time
  const remainder = months % 12 >= 6 ? ".5" : "";

  // add the remainder to years only if it's less than 3 years
  let years = "";
  if (months < 12) {
    years = "< 1";
  } else if (months < 36) {
    years = Math.floor(months / 12).toString() + remainder;
  } else {
    years = Math.floor(months / 12).toString();
  }

  return years + "y";
}

/** Returns a simplified number, e.g 1,327 -> 1.3K. */
function shorten_num(v?: number) {
  if (!v) return "0";

  let txt = "";
  if (v < 1000) {
    txt = "< 1K";
  } else if (v >= 1_000 && v < 10_000.0) {
    txt = `${Math.round(v / 1000).toPrecision(1)}K`;
  } else if (v >= 10_000 && v < 1_000_000) {
    txt = `${v / 1000}K`;
  } else {
    txt = `${Math.round(v / 1_000_000.0).toPrecision(1)}M`;
  }

  return txt;
}

/** Formats integer numbers with commas for readability, e.g. 100000 -> 10,000. */
function pretty_num(v?: number) {
  if (!v) return "";

  return Math.round(v)
    .toString()
    .replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

/** Adds or removes a dev from the shortlist */
const toggleShortlistStatus = (newStatus: boolean) => {
  // console.log(`toggle: ${newStatus}`);
  // console.log(props.devDetails);
  if (newStatus) {
    store.shortlist.set(devId.value, props.devDetails);
  } else {
    store.shortlist.delete(devId.value);
  }
};

/** Returns TRUE if the dev is present in store.shortlist. */
const isShortlisted = computed(() => {
  const has = store.shortlist.has(devId.value);
  // console.log(`${devId.value}: ${has}`);
  return has;
});
</script>

<template>
  <div class="card mb-4">
    <div class="card-body">
      <div
        class="d-flex justify-content-between align-items-center"
        width="100%"
        height="50"
      >
        <h5 class="card-title ma-1">
          <input
            type="checkbox"
            :checked="isShortlisted"
            :id="devId"
            :value="devId"
            @change="(event) => toggleShortlistStatus((event.target as HTMLInputElement).checked)"
            class="me-2"
          />
          <a :href="devId">
            {{ devName }}
          </a>
        </h5>
      </div>

      <p class="card-subtitle mb-2">
        <span class="me-2 text-muted">
          {{ projectsOverYears }}
        </span>
        <a
          v-if="hasPublicContactDetails"
          href="https://github.com/{{dev._source.login}}"
          title="Contact details available on GitHub"
          ><span class="badge bg-success">Contact</span></a
        >
        <span v-if="props.devDetails.location" class="ms-1">{{
          props.devDetails.location
        }}</span>
      </p>

      <ul class="list-inline">
        <li
          v-for="tech in matchingLanguages"
          :key="tech.language"
          class="list-inline-item bg-light text-dark py-1 px-2 rounded mb-3 me-3 border border-success"
        >
          <h6 class="mb-1">{{ tech.language }}</h6>
          <span class="fw-light smaller-90">
            <span class="calendar-badge me-3">{{
              months_to_years(tech.history?.months)
            }}</span>
            <span class="loc-badge">{{ shorten_num(tech?.codeLines) }}</span>
          </span>
        </li>

        <li
          v-for="pkg in matchingPkgs"
          :key="pkg.k"
          class="list-inline-item bg-light text-dark py-1 px-2 rounded mb-3 me-3 border border-success"
        >
          <h6 class="mb-1">{{ pkg.k }}</h6>
          <span class="fw-light smaller-90">
            <span class="libs-badge">
              {{ pretty_num(pkg.c) }} mention{{ pkg.c > 1 ? "s" : "" }}
            </span>
          </span>
        </li>

        <li
          v-for="tech in otherLanguages"
          :key="tech.language"
          class="list-inline-item bg-light text-dark py-1 px-2 rounded mb-3 me-3 border"
        >
          <h6 class="mb-1 fw-light">{{ tech.language }}</h6>
          <span class="fw-light smaller-90">
            <span class="calendar-badge me-3">
              {{ months_to_years(tech.history?.months) }}
            </span>
            <span class="loc-badge">{{ shorten_num(tech.codeLines) }}</span>
          </span>
        </li>
      </ul>
    </div>
  </div>
</template>
