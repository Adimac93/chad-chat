<script lang="ts">
  import Login from "./Login.svelte";
  import Register from "./Register.svelte";

  export let menu;
</script>

<div class="card">
  {#if menu == "login"}
    <Login />
  {:else if menu == "register"}
    <Register />
  {/if}
</div>
