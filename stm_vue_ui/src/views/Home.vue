<template>
  <div class="home">
    <div class="container-fluid">
      <div class="row justify-content-center">
        <div class="col-lg-8 col-md-10 col-xs-12 col-xxl-6">
          <HelloWorld msg="Open Directory of Software Developers" />
          <button
            :disabled="auth.authenticated"
            @click="auth.loginWithRedirect({scope: 'email profile id_token'})"
          >
            Login with Redirect
          </button>
          <button
            :disabled="!auth.authenticated"
            @click="auth.logout()"
          >
            Logout
          </button>
          <button
            :disabled="!auth.authenticated"
            @click="jwt()"
          >
            Token
          </button>

          <div class="status">
            <span>{{ auth.authenticated ? 'Authenticated' : 'Not authenticated' }}</span>
            <span>{{ auth.loading ? 'Loading' : 'Not loading' }}</span>
            <span>User: {{ auth.user?.name || 'NO_USER' }}</span>
            <span>Email: {{ auth.user?.email || 'NO_EMAIL' }}</span>
          </div>

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
          </p> <h6 class="mt-5 mb-3">
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
          </ul> <h6 class="mt-5">
            About StackMuncher
          </h6> <p>
            StackMuncher helps software developers find better jobs that match their technology stack and interests. The stack data comes from analyses of public and private Git repositories via <a
              title="StackMuncher developer profile builder app"
              href="https://github.com/stackmuncher/stm_app"
            >StackMuncher App</a>. It is a community-focused open source project.
          </p>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
// @ is an alias to /src
import HelloWorld from '@/components/HelloWorld.vue'
import { devsPerLanguageQuery } from '@/graphql/queries.ts'

export default {
  name: 'Home',
  components: {
    HelloWorld
  },
  inject: ['auth'],
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
