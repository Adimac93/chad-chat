<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { createNewGroup } from "../api/groups";

  const dispatch = createEventDispatcher<{ groupCreate }>();
  let isOk;
  let groupName = "";
  let prompt = "";
  async function create() {
    if (groupName.length == 0) return;
    isOk = await createNewGroup(groupName);

    if (isOk) {
      prompt = `succesfuly created group "${groupName}"`;
      setTimeout(() => {
        prompt = "";
      }, 5000);
      dispatch("groupCreate");
    }
    groupName = "";
  }
</script>

<form>
  <input bind:value={groupName} placeholder="Chad name" type="text" />
  <button on:click|preventDefault={create}>Create group</button>
  <div style="color: grey">{prompt}</div>
</form>

<style>
  form {
    display: inline;
    align-items: right;
  }
</style>
