<script lang="ts">
  import Auth from "./lib/auth/Auth.svelte";
  import Logout from "./lib/auth/Logout.svelte";
  import Chat from "./lib/chat/Chat.svelte";
  import ChatBox from "./lib/chat/ChatBox.svelte";
  import { isAuthorized } from "./lib/stores";

  let menu: "login" | "register" | "chat" = "login";
  let messages: Array<MessageModel> = [
    { content: "Hello", sat: 100000, sender: "it's me mario" },
    { content: "Hello", sat: 100000, sender: "it's me mario" },
    { content: "Hello", sat: 100000, sender: "it's me mario" },
  ];
</script>

{#if !$isAuthorized}
  <nav class="card">
    <button on:click={() => (menu = "login")}>Login</button>
    <button on:click={() => (menu = "register")}>Register</button>
  </nav>
{/if}

<main>
  {#if $isAuthorized}
    <Logout />
    <Chat />
  {:else}
    <Auth bind:menu />
  {/if}
</main>
