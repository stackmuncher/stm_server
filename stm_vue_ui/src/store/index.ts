import { InjectionKey } from 'vue'
import { createStore, Store, useStore as baseUseStore } from 'vuex'
import * as mutNames from './mutations'

export type Lang = Map<string, number>

// define your typings for the store state
export interface State {
  filterLang: Lang,
  resultCount: number,
  jwt: string | null,
  mutations: {
    [mutNames.addLang] (state: State, lang: string, expertise: number): void,
    [mutNames.removeLang] (state: State, lang: string): void,
    [mutNames.setJWT] (state: State, jwt: string): void
  }
}

// define injection key
export const key: InjectionKey<Store<State>> = Symbol('InjectionKey<Store<State>>')

export const store = createStore<State>({
  state: {
    filterLang: new Map<string, number>(),
    resultCount: 0,
    jwt: null,
    mutations: {
      [mutNames.addLang]: addLang,
      [mutNames.removeLang]: removeLang,
      [mutNames.setJWT]: setJWT
    }
  }
})

// define your own `useStore` composition function
export function useStore () {
  return baseUseStore(key)
}

/** Adds a language to the list of filters. */
function addLang (state: State, lang: string, expertise: number): void {
  state.filterLang.set(lang, expertise)
}

/** Removes a language from the list of filters. */
function removeLang (state: State, lang: string): void {
  state.filterLang.delete(lang)
}

/** Adds a language to the list of filters. */
function setJWT (state: State, jwt: string): void {
  state.jwt = jwt
}
