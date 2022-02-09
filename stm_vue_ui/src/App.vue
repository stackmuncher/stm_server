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
    </div>
  </nav>

  <router-view />
  <nav class="navbar navbar-expand-lg navbar-light mx-0 mb-3 mt-5 border-top">
    <div class="container-fluid">
      <div
        id="navbarSupportedContent"
        class="collapse navbar-collapse show"
      >
        <div class="navbar-nav ms-md-5 me-auto justify-content-center">
          <div
            style="height: 2em"
            class="d-inline-block"
          >
            <a
              href="https://github.com/stackmuncher"
              target="blank"
              title="StackMuncher code on GitHub"
            >
              <img
                class="h-75 me-2"
                src="https://stackmuncher.com/about/images/social/github.svg"
                alt="Github"
              >
            </a>
            <a
              href="https://www.linkedin.com/in/maxvoskob/"
              target="blank"
              title="LinkedIn of the founder"
            >
              <img
                class="h-75 me-2"
                src="https://stackmuncher.com/about/images/social/linkedin.svg"
                alt="LinkedIn"
              >
            </a>
            <a
              href="/_stats"
              target="blank"
              title="StackMuncher status page"
            >
              <img
                class="h-75 me-2"
                src="https://stackmuncher.com/about/images/social/heartbeat.svg"
                alt="Status Page"
              >
            </a>
            <a
              href="http://creativecommons.org/licenses/by/4.0/"
              target="blank"
              title="All site content is licensed under Creative Commons License"
            >
              <img
                class="h-75 me-2"
                src="https://stackmuncher.com/about/images/social/cc-by.svg"
                alt="Creative Commons License"
              >
            </a>
          </div>
        </div> <div class="d-flex my-2 my-lg-0 text-center">
          <a href="mailto:info@stackmuncher.com">info@stackmuncher.com</a>
        </div>
      </div>
    </div>
  </nav>
</template>

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
