<script lang="ts">
  import Login from "./Login.svelte";
  import Register from "./Register.svelte";

  export let type: "login"| "register";
</script>

<div class="card">
  {#if type == "login"}
    <Login />
  {:else if type == "register"}
    <Register />
  {/if}
</div>
