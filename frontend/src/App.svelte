<script lang="ts">
  import Auth from "./lib/auth/Auth.svelte";
  import Logout from "./lib/auth/Logout.svelte";
  import Register from "./lib/auth/Register.svelte";
  import Chat from "./lib/chat/Chat.svelte";
  import { isAuthorized } from "./lib/stores";
  import TextButton from "./lib/TextButton.svelte";

  let menu = "";
  const menuTypes = ["Chat", "Friends"];

  let auth: "login" | "register" = "login";
</script>

{#if !$isAuthorized}
  <nav class="card">
    {#if auth == "login"}
      <div>Don't have account?</div>
      <TextButton on:click={() => (auth = "register")}>Register</TextButton>
    {:else}
      <div>Already a chad?</div>
      <TextButton on:click={() => (auth = "login")}>Login</TextButton>
    {/if}
  </nav>
{/if}

<main>
  {#if $isAuthorized}
    <Logout />
    {#if menu == "Chat"}
      <Chat />
    {:else}
      <div>
        {#each menuTypes as menuType}
          <button on:click={() => (menu = menuType)}>{menuType}</button>
        {/each}
      </div>
    {/if}
  {:else}
    <Auth bind:type={auth} />
  {/if}
</main>
