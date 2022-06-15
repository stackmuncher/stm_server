<script setup lang="ts">
import FooterBlock from "@/components/FooterBlock.vue";
import LinkedInLogin from "@/components/LinkedInLogin.vue";
import { provide } from "vue";
import { RouterView, RouterLink } from "vue-router";
import { AuthenticationProperties as auth0 } from "vue-auth0-plugin";
import {
  ApolloClient,
  InMemoryCache,
  createHttpLink,
} from "@apollo/client/core";
import { setContext } from "@apollo/client/link/context";
import { DefaultApolloClient } from "@vue/apollo-composable";

// middleware required to inject a JWT into every GQL request
// see https://github.com/jnt0r/vue-auth0-plugin, https://github.com/apollographql/apollo-client/issues/2441, https://www.apollographql.com/docs/react/networking/authentication/
const authLink = setContext(async (_, { headers }) => {
  const token = await auth0.getIdTokenClaims();
  return {
    headers: {
      ...headers,
      authorization: token ? `Bearer ${token.__raw}` : "",
    },
  };
});

const httpLink = createHttpLink({
  uri: "https://jv4ztf9od8.execute-api.us-east-1.amazonaws.com",
});

const apolloClient = new ApolloClient({
  cache: new InMemoryCache(),
  link: authLink.concat(httpLink),
  connectToDevTools: true,
});

provide(DefaultApolloClient, apolloClient);
</script>

<template>
  <nav
    class="navbar navbar-expand-lg navbar-light bg-light mb-4 px-sm-3"
    role="navigation"
  >
    <div class="container-fluid">
      <a
        class="navbar-brand"
        title="Home"
        style="
          background: left/auto 80% no-repeat;
          background-image: url(https://stackmuncher.com/about/logo/logo-color.svg);
          padding-left: 55px;
        "
        href="/"
        >Stack Muncher</a
      >
      <div class="collapse navbar-collapse">
        <ul class="navbar-nav ms-auto me-auto mb-2 mb-lg-0">
          <li class="nav-item">
            <router-link to="/" class="nav-link">Home</router-link>
          </li>
          <li class="nav-item">
            <router-link to="/about" class="nav-link"> About </router-link>
          </li>
        </ul>

        <LinkedInLogin />
      </div>
    </div>
  </nav>
  <router-view />
  <FooterBlock />
</template>

<style>
@import "https://cdn.jsdelivr.net/npm/bootstrap@5.2.0-beta1/dist/css/bootstrap.min.css";

body {
  font-size: 14px;
}

strong,
th {
  font-weight: 600;
}

dt {
  font-weight: normal;
}

.nav-link {
  font-size: 16px;
}

.loc-badge {
  background-image: url("https://assets.stackmuncher.com/icons/lines_of_code.svg");
  background-clip: padding-box;
  background-position: left;
  background-repeat: no-repeat;
  padding-left: 1.2rem;
}

.libs-badge {
  background-image: url("https://assets.stackmuncher.com/icons/libraries.svg");
  background-clip: padding-box;
  background-position: left;
  background-repeat: no-repeat;
  padding-left: 1.2rem;
}

.calendar-badge {
  background-image: url("https://assets.stackmuncher.com/icons/calendar.svg");
  background-clip: padding-box;
  background-position: left;
  background-repeat: no-repeat;
  padding-left: 1.2rem;
}

.team-badge {
  background-image: url("https://assets.stackmuncher.com/icons/team.svg");
  background-clip: padding-box;
  background-position: left;
  background-repeat: no-repeat;
  padding-left: 1.7rem;
}

.commits-badge {
  background-image: url("https://assets.stackmuncher.com/icons/commits.svg");
  background-clip: padding-box;
  background-position: left;
  background-repeat: no-repeat;
  padding-left: 1rem;
}

.smaller-90 {
  font-size: 90%;
}

.svg-icon {
  width: 2em;
  height: 1em;
  vertical-align: -0.125em;
}

.clickable {
  cursor: pointer;
}
</style>
