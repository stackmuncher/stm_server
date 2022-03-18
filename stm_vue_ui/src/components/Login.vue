<template>
  <button
    v-if="!auth.authenticated"
    @click="auth.loginWithRedirect({scope: 'email profile id_token'})"
  >
    Login with Redirect
  </button>
  <button
    v-if="auth.authenticated"
    :title="auth.user?.name + ' / ' + auth.user?.email"
    @click="auth.logout()"
  >
    Logout
  </button>
</template>

<script>

export default {
  name: 'Login',
  inject: ['auth']
}
</script>
