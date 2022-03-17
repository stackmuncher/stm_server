<template>
  <div class="home">
    <div class="container-fluid">
      <div class="row justify-content-center">
        <div class="col-lg-8 col-md-10 col-xs-12 col-xxl-6">
          <h1>Reach out to software developers</h1>
          <DevSearch />
          <DevCounts />
          <AboutStm />
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import DevCounts from '@/components/DevCounts.vue'
import DevSearch from '@/components/DevSearch.vue'
import AboutStm from '@/components/explainers/AboutStm.vue'

export default {
  name: 'Home',
  components: {
    DevCounts,
    DevSearch,
    AboutStm
  }
}

</script>
