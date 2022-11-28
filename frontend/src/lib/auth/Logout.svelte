<script lang="ts">
  import { isAuthorized, messages } from "../stores";

  function deAuthorize() {
    isAuthorized.set(false);
    messages.set([]);
  }
</script>

<button on:click={deAuthorize}>Logout</button>
