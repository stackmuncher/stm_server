import { createApp } from 'vue'
import App from './App.vue'
import store from './store'
import router from './router'
import VueAuth0Plugin from 'vue-auth0-plugin'
import { ApolloClient, InMemoryCache, createHttpLink } from '@apollo/client/core'
import { setContext } from '@apollo/client/link/context'
import { createApolloProvider } from '@vue/apollo-option'

const httpLink = createHttpLink({
  uri: 'https://jv4ztf9od8.execute-api.us-east-1.amazonaws.com'
})

const authLink = setContext((_, { headers }) => {
  // get the authentication token from local storage if it exists
  const token = 'jwt-token' // localStorage.getItem('jwt')
  // return the headers to the context so httpLink can read them
  return {
    headers: {
      ...headers,
      authorization: token ? `Bearer ${token}` : ''
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
