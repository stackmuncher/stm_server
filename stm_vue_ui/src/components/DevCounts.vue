<template>
  <h6 class="mt-5 mb-3">
    Developers per language
  </h6> <ul
    v-if="devsPerLanguage"
    class="list-inline"
  >
    <li
      v-for="bucket in devsPerLanguage.aggregations.agg.buckets"
      :key="bucket.key"
      class="list-inline-item bg-light text-dark p-1 rounded mb-3 border me-4"
    >
      <a
        :title="`${bucket.key} developers`"
        style="text-decoration: underline #6c757d;"
        class="text-dark"
        href="/?{{ bucket.key }}"
      > {{ bucket.key }} <span
        class="badge bg-white text-dark ms-2"
        style="font-weight: 300;"
      >{{ bucket.docCount }}</span></a>
    </li>
  </ul>
</template>

<script>
import { devsPerLanguageQuery } from '@/graphql/queries.ts'

export default {
  name: 'DevCounts',
  data () {
    return {
      devsPerLanguage: null,
      loading: 0
    }
  },
  computed: {

  },
  apollo: {
    devsPerLanguage: devsPerLanguageQuery
  }

}

</script>
