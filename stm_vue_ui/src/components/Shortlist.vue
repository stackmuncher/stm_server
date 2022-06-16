<script setup lang="ts">
import { useQueryStore } from "@/stores/QueryStore";
import DevCard from "./DevCard.vue";

const store = useQueryStore();
</script>

<template>
  <h2 class="pe-md-5 text-muted" v-if="store.shortlist.size == 0">
    The shortlist is empty
  </h2>
  <div v-for="dev in store.shortlist.values()" :key="dev.login">
    <DevCard :dev-details="dev" />
  </div>
</template>
