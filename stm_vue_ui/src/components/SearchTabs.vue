<script setup lang="ts">
import { useQueryStore, SearchTabNames } from "@/stores/QueryStore";

const store = useQueryStore();
</script>

<template>
  <ul class="nav nav-tabs mb-3">
    <li class="nav-item">
      <span
        title="Modify search criteria to get a list of developers matching your needs."
        class="nav-link text-secondary clickable"
        :class="{ active: store.activeSearchTab == SearchTabNames.Search }"
        @click="store.activeSearchTab = SearchTabNames.Search"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          class="svg-icon"
          aria-hidden="true"
          focusable="false"
          viewBox="0 0 512 512"
        >
          <path
            fill="currentColor"
            d="M500.3 443.7l-119.7-119.7c27.22-40.41 40.65-90.9 33.46-144.7C401.8 87.79 326.8 13.32 235.2 1.723C99.01-15.51-15.51 99.01 1.724 235.2c11.6 91.64 86.08 166.7 177.6 178.9c53.8 7.189 104.3-6.236 144.7-33.46l119.7 119.7c15.62 15.62 40.95 15.62 56.57 0C515.9 484.7 515.9 459.3 500.3 443.7zM79.1 208c0-70.58 57.42-128 128-128s128 57.42 128 128c0 70.58-57.42 128-128 128S79.1 278.6 79.1 208z"
          />
        </svg>
        Search</span
      >
    </li>
    <li class="nav-item">
      <span
        title="View profiles of developers matching your search."
        class="nav-link text-secondary clickable"
        :class="{ active: store.activeSearchTab == SearchTabNames.Profiles }"
        @click="store.activeSearchTab = SearchTabNames.Profiles"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          class="svg-icon"
          aria-hidden="true"
          focusable="false"
          viewBox="0 0 512 512"
        >
          <path
            fill="currentColor"
            d="M319.9 320c57.41 0 103.1-46.56 103.1-104c0-57.44-46.54-104-103.1-104c-57.41 0-103.1 46.56-103.1 104C215.9 273.4 262.5 320 319.9 320zM369.9 352H270.1C191.6 352 128 411.7 128 485.3C128 500.1 140.7 512 156.4 512h327.2C499.3 512 512 500.1 512 485.3C512 411.7 448.4 352 369.9 352zM512 160c44.18 0 80-35.82 80-80S556.2 0 512 0c-44.18 0-80 35.82-80 80S467.8 160 512 160zM183.9 216c0-5.449 .9824-10.63 1.609-15.91C174.6 194.1 162.6 192 149.9 192H88.08C39.44 192 0 233.8 0 285.3C0 295.6 7.887 304 17.62 304h199.5C196.7 280.2 183.9 249.7 183.9 216zM128 160c44.18 0 80-35.82 80-80S172.2 0 128 0C83.82 0 48 35.82 48 80S83.82 160 128 160zM551.9 192h-61.84c-12.8 0-24.88 3.037-35.86 8.24C454.8 205.5 455.8 210.6 455.8 216c0 33.71-12.78 64.21-33.16 88h199.7C632.1 304 640 295.6 640 285.3C640 233.8 600.6 192 551.9 192z"
          />
        </svg>
        Profiles
      </span>
    </li>
    <li class="nav-item">
      <span
        title="A shortlist of matching developers to notify about your job or project."
        class="nav-link text-secondary clickable"
        :class="{ active: store.activeSearchTab == SearchTabNames.Shortlist }"
        @click="store.activeSearchTab = SearchTabNames.Shortlist"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          class="svg-icon"
          aria-hidden="true"
          focusable="false"
          viewBox="0 0 512 512"
        >
          <path
            fill="currentColor"
            d="M152.1 38.16C161.9 47.03 162.7 62.2 153.8 72.06L81.84 152.1C77.43 156.9 71.21 159.8 64.63 159.1C58.05 160.2 51.69 157.6 47.03 152.1L7.029 112.1C-2.343 103.6-2.343 88.4 7.029 79.03C16.4 69.66 31.6 69.66 40.97 79.03L63.08 101.1L118.2 39.94C127 30.09 142.2 29.29 152.1 38.16V38.16zM152.1 198.2C161.9 207 162.7 222.2 153.8 232.1L81.84 312.1C77.43 316.9 71.21 319.8 64.63 319.1C58.05 320.2 51.69 317.6 47.03 312.1L7.029 272.1C-2.343 263.6-2.343 248.4 7.029 239C16.4 229.7 31.6 229.7 40.97 239L63.08 261.1L118.2 199.9C127 190.1 142.2 189.3 152.1 198.2V198.2zM224 96C224 78.33 238.3 64 256 64H480C497.7 64 512 78.33 512 96C512 113.7 497.7 128 480 128H256C238.3 128 224 113.7 224 96V96zM224 256C224 238.3 238.3 224 256 224H480C497.7 224 512 238.3 512 256C512 273.7 497.7 288 480 288H256C238.3 288 224 273.7 224 256zM160 416C160 398.3 174.3 384 192 384H480C497.7 384 512 398.3 512 416C512 433.7 497.7 448 480 448H192C174.3 448 160 433.7 160 416zM0 416C0 389.5 21.49 368 48 368C74.51 368 96 389.5 96 416C96 442.5 74.51 464 48 464C21.49 464 0 442.5 0 416z"
          />
        </svg>
        Shortlist</span
      >
    </li>
    <li class="nav-item">
      <span
        title="Describe the opportunity"
        class="nav-link text-secondary clickable"
        :class="{ active: store.activeSearchTab == SearchTabNames.Message }"
        @click="store.activeSearchTab = SearchTabNames.Message"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          class="svg-icon"
          aria-hidden="true"
          focusable="false"
          viewBox="0 0 512 512"
        >
          <path
            fill="currentColor"
            d="M0 64C0 28.65 28.65 0 64 0H224V128C224 145.7 238.3 160 256 160H384V299.6L289.3 394.3C281.1 402.5 275.3 412.8 272.5 424.1L257.4 484.2C255.1 493.6 255.7 503.2 258.8 512H64C28.65 512 0 483.3 0 448V64zM256 128V0L384 128H256zM564.1 250.1C579.8 265.7 579.8 291 564.1 306.7L534.7 336.1L463.8 265.1L493.2 235.7C508.8 220.1 534.1 220.1 549.8 235.7L564.1 250.1zM311.9 416.1L441.1 287.8L512.1 358.7L382.9 487.9C378.8 492 373.6 494.9 368 496.3L307.9 511.4C302.4 512.7 296.7 511.1 292.7 507.2C288.7 503.2 287.1 497.4 288.5 491.1L303.5 431.8C304.9 426.2 307.8 421.1 311.9 416.1V416.1z"
          />
        </svg>
        Job description</span
      >
    </li>
  </ul>
</template>
