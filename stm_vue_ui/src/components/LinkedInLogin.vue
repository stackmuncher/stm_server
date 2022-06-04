<script setup lang="ts">
import { injectAuth } from "vue-auth0-plugin";

const auth = injectAuth();
</script>

<template>
  <div class="d-flex">
    <button
      v-if="auth && auth.authenticated"
      :title="auth?.user?.name + ' / ' + auth?.user?.email"
      @click="auth?.logout()"
    >
      Logout
    </button>
  </div>
</template>
