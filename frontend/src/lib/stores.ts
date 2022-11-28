import { writable } from "svelte/store";

export const isAuthorized = writable<boolean>(localStorage.getItem("isAuthorized") === 'true')
isAuthorized.subscribe((value) => localStorage.isAuthorized = String(value))

export const messages = writable<Array<MessageModel>>([])