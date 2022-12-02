<script lang="ts">
  import { afterUpdate } from "svelte";

  let name;
  let members;

  let code = "";
  let error = "";

  let isCorrect = false;

  async function getGroupInfo() {
    let res = await fetch(`/api/groups/invitations/info`, {
      method: "POST",
      headers: { "Content-type": "application/json" },
      body: JSON.stringify({ code }),
    });
    if (res.ok) {
      isCorrect = true;
      const json = await res.json();
      name = json.name as string;
      members = json.members as number;
    } else if (res.status == 400) {
      error = (await res.json()).error_info;
      setTimeout(() => {
        error = "";
      }, 5000);
      code = "";
    }
  }
  async function joinGroup() {
    let res = await fetch(`/api/groups/invitations/join`, {
      method: "POST",
      headers: { "Content-type": "application/json" },
      body: JSON.stringify({ code }),
    });
    if (res.ok) {
    } else {
      const json = await res.json();
    }
  }
  $: checkCode(code);

  const checkCode = (text: string) => {
    code = text.replaceAll(" ", "");
    if (code.length == 10) {
      getGroupInfo();
    }
  };
</script>

<div class="card">
  {#if isCorrect}
    <strong>Group: {name}</strong>
    <div>Members: {members}</div>
    <button on:click={joinGroup}>Join</button>
  {/if}
  <div>Enter group join code:</div>
  <input disabled={isCorrect} bind:value={code} type="text" />
  <div>{error}</div>
</div>

<style>
  div {
    margin: 20px;
  }
  input {
    text-align: center;
  }
</style>
