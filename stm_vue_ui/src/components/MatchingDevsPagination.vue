<script setup lang="ts">
import { computed } from "vue";
import { useQueryStore, PROFILES_PER_PAGE } from "@/stores/QueryStore";

const store = useQueryStore();

const pageNumList = computed(() =>
  [
    ...Array(
      Math.min(20, Math.ceil(store.searchProfileCount / PROFILES_PER_PAGE))
    ).keys(),
  ].map((x) => ++x)
);

/** store.currentPageProfiles is zero-based and needs to be incremented before use in UI. */
const currentPageProfiles = computed(() => store.currentPageProfiles + 1);

const lastPage = computed(() => {
  // console.log(
  //   `pages: ${Math.ceil(store.searchProfileCount / PROFILES_PER_PAGE)}`
  // );
  // console.log(`current: ${store.currentPageProfiles}`);

  return Math.ceil(store.searchProfileCount / PROFILES_PER_PAGE);
});

/** Forces an update with data for the specified page. */
const navigateToPage = (p: number) => {
  // console.log(`navigating to: ${p}`);
  store.currentPageProfiles = Math.max(0, p - 1);
};
</script>

<template>
  <div class="row justify-content-center mt-4" v-if="lastPage > 1">
    <div class="col-lg-10 col-xxl-8">
      <ul class="list-inline text-center">
        <li
          class="list-inline-item"
          v-for="pageNum in pageNumList"
          :key="pageNum"
        >
          <span
            @click="navigateToPage(pageNum)"
            :class="{
              clickable: pageNum != currentPageProfiles,
              'text-decoration-underline': pageNum != currentPageProfiles,
            }"
            >{{ pageNum }}</span
          >
        </li>
      </ul>
    </div>
  </div>
</template>
