<script lang="ts">
  import Popup from "../../Popup.svelte";
  import CreateInvitation from "./CreateInvitation.svelte";
  import UseInvitation from "./UseInvitation.svelte";
  let mode: "create" | "join";

  export let group_id: string;
</script>

<Popup on:close={() => (mode = undefined)}>
  {#if mode == "create"}
    <CreateInvitation bind:group_id />
  {:else if mode == "join"}
    <UseInvitation />
  {:else}
    <div class="card">
      <button on:click={() => (mode = "create")}>Create</button>
      <button on:click={() => (mode = "join")}>Join</button>
    </div>
  {/if}
</Popup>
