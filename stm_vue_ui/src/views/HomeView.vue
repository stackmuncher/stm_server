<script setup lang="ts">
import DevsPerTech from "@/components/DevsPerTech.vue";
import DevSearch from "@/components/DevSearch.vue";
import AboutStm from "@/components/explainers/AboutStm.vue";
import MatchingDevsCount from "@/components/MatchingDevsCount.vue";
import { useQueryStore } from "@/stores/QueryStore";

const store = useQueryStore();
</script>

<template>
  <div class="home">
    <div class="container-fluid">
      <div class="row justify-content-center">
        <div class="col-lg-8 col-md-10 col-xs-12 col-xxl-6">
          <h1>Reach out to software developers</h1>
          <MatchingDevsCount v-if="store.techListLoaded" />
          <DevsPerTech />
          <DevSearch />
          <AboutStm />
        </div>
      </div>
    </div>
  </div>
</template>
