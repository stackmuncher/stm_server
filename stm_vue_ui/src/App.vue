<template>
  <nav class="navbar navbar-expand-lg navbar-light bg-light mb-4 px-sm-3">
    <div class="container-fluid">
      <a
        class="navbar-brand"
        title="Home"
        style="background: left/auto 80% no-repeat url(https://stackmuncher.com/about/logo/logo-color.svg); padding-left: 55px;"
        href="/"
      >Stack Muncher</a>
      <router-link to="/">
        Home
      </router-link> |
      <router-link to="/about">
        About
      </router-link>
      <Login />
    </div>
  </nav>

  <router-view />
  <Footer />
</template>

<script lang="ts">
import Footer from '@/components/Footer.vue'
import Login from '@/components/Login.vue'

export default {
  name: 'App',
  components: {
    Footer,
    Login
  }
}
</script>

<style>
  @import "https://cdn.jsdelivr.net/npm/bootstrap@5.1.3/dist/css/bootstrap.min.css";
   body { font-size: 14px }
   strong, th { font-weight: 600; }
   dt { font-weight: normal; }
   .nav-link { font-size: 16px; }
   .loc-badge { background-image: url("https://assets.stackmuncher.com/icons/lines_of_code.svg"); background-clip: padding-box; background-position: left; background-repeat: no-repeat; padding-left: 1.2rem; }
   .libs-badge { background-image: url("https://assets.stackmuncher.com/icons/libraries.svg"); background-clip: padding-box; background-position: left; background-repeat: no-repeat; padding-left: 1.2rem; }
   .calendar-badge { background-image: url("https://assets.stackmuncher.com/icons/calendar.svg"); background-clip: padding-box; background-position: left; background-repeat: no-repeat; padding-left: 1.2rem; }
   .team-badge { background-image: url("https://assets.stackmuncher.com/icons/team.svg"); background-clip: padding-box; background-position: left; background-repeat: no-repeat; padding-left: 1.5rem; }
   .commits-badge { background-image: url("https://assets.stackmuncher.com/icons/commits.svg"); background-clip: padding-box; background-position: left; background-repeat: no-repeat; padding-left: 1rem; }
   .smaller-90 { font-size: 90%; }
</style>
