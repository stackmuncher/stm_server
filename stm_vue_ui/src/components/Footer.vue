<template>
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

<script lang="ts">
import { Vue } from 'vue-class-component'

export default class Footer extends Vue {}
</script>
