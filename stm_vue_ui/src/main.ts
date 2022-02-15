import { createApp } from 'vue'
import App from './App.vue'
import store from './store'
import router from './router'
import VueAuth0Plugin, { AuthenticationProperties as auth0 } from 'vue-auth0-plugin'
import { ApolloClient, InMemoryCache, createHttpLink } from '@apollo/client/core'
import { setContext } from '@apollo/client/link/context'
import { createApolloProvider } from '@vue/apollo-option'

const httpLink = createHttpLink({
  uri: 'https://jv4ztf9od8.execute-api.us-east-1.amazonaws.com'
})

// middleware required to inject a JWT into every GQL request
// see https://github.com/jnt0r/vue-auth0-plugin, https://github.com/apollographql/apollo-client/issues/2441, https://www.apollographql.com/docs/react/networking/authentication/
const authLink = setContext(async (_, { headers }) => {
  const token = await auth0.getIdTokenClaims()
  return {
    headers: {
      ...headers,
      authorization: token ? `Bearer ${token.__raw}` : ''
    }
  }
})

const apolloClient = new ApolloClient({
  cache: new InMemoryCache(),
  link: authLink.concat(httpLink),
  connectToDevTools: true
})

const apolloProvider = createApolloProvider({
  defaultClient: apolloClient
})

createApp(App).use(router).use(store).use(VueAuth0Plugin, {
  domain: 'stackmuncher.us.auth0.com',
  client_id: 'Zf2S4CkHRe9M7l74J1AjDgaxYuooujH0',
  redirect_uri: 'http://localhost:8080/about',
  scope: 'email profile id_token'
  // ... other optional options ...
}).use(apolloProvider).mount('#app')
