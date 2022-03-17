<template>
  <div class="d-flex mt-4">
    <input
      id="kw"
      class="form-control me-2"
      type="search"
      title="Find devs working with a particular language, package or API, e.g. C# + Twilio + Azure."
      value=""
      minlength="1"
      maxlength="100"
      onkeydown="if (event.keyCode===13) document.getElementById('btn').click()"
    > <button
      id="btn"
      class="btn btn-success my-2 my-sm-0"
      type="button"
      autocomplete="off"
      onclick="const v=document.getElementById('kw'); if (v.validity.valid) window.location.href='/?'+encodeURIComponent(v.value)"
    >
      Search
    </button>
  </div> <p class="mb-5 text-muted mt-2">
    Find software developers by their technology stack, e.g. <code>typescript vuejs apollo</code> or <code>c# sql cosmos</code>
  </p>
</template>

<script lang="ts">
import { Vue } from 'vue-class-component'

export default class Footer extends Vue {}
</script>
