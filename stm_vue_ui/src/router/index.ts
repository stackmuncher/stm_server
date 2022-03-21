import { createRouter, createWebHistory } from "vue-router";
import HomeView from "../views/HomeView.vue";
import AboutView from "../views/AboutView.vue";
import { AuthenticationState } from "vue-auth0-plugin";
import { AuthenticationProperties as auth } from "vue-auth0-plugin";

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: "/",
      name: "home",
      component: HomeView,
    },
    {
      path: "/about",
      name: "about",
      component: AboutView,
    },
  ],
});

// only authenticated users may access this Vue app
// the auth should apply to the entire site, not just select routes
router.beforeEach(async () => {
  // an early exit if the user has been auth'd earlier
  if (AuthenticationState && AuthenticationState.authenticated) {
    return true;
  }

  // it could that the authentication is still in progress
  // the problem with this call is that it never returns if authenticated
  const isAuthed = await AuthenticationState.getAuthenticatedAsPromise();

  // try to log in if not authenticated up to this point
  if (!isAuthed) {
    // this redirects to Auth0 login form, but sometimes it just logs me in
    // even if I delete all auth0 cookies
    auth.loginWithRedirect();
    // the above line should be wrapped into an error checker
    // in case it's a callback from a failed login, e.g. the user pressed cancel on LinkedIn login page
    // as of now the code will redirect back to LiN login page again in an infinite loop
    // see https://github.com/jnt0r/vue-auth0-plugin/issues/418
  }

  // recheck if the user was logged in successfully
  // redirect to a public welcome page on failure
  if (!auth || !auth.authenticated) {
    location.assign("https://stackmuncher.com");
  }

  // allow the navigation
  return true;
});

export default router;
