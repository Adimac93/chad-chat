<script lang="ts">
  import Popup from "../../Popup.svelte";
  export let group_id;

  const expirationChoice = [
    "30 minutes",
    "1 hour",
    "6 hours",
    "12 hours",
    "1 day",
    "Never",
  ];
  const usesChoices = [
    "1 use",
    "5 uses",
    "10 uses",
    "25 uses",
    "50 uses",
    "100 uses",
    "No limit",
  ];

  let expiration_index;
  let usses_index;
  let code = "";

  async function copyToClipboard(text: string) {
    await navigator.clipboard.writeText(text);
  }

  async function createGroupInvitation() {
    let res = await fetch(`/api/groups/invitations/create`, {
      method: "POST",
      headers: { "Content-type": "application/json" },
      body: JSON.stringify({
        group_id,
        expiration_index,
        usses_index,
      }),
    });
    if (res.ok) {
      const json = await res.json();
      code = json.code as string;
      await copyToClipboard(code);
    }
  }
</script>

<Popup>
  <div class="card">
    <div>Expire after</div>
    <select bind:value={expiration_index}>
      {#each expirationChoice as exp, i}
        <option value={i}>{exp}</option>
      {/each}
    </select>

    <div>Max number of uses</div>
    <select bind:value={usses_index}>
      {#each usesChoices as uses, i}
        <option value={i}>{uses}</option>
      {/each}
    </select>
    <br />
    {#if code}
      <div>{code}</div>
      <div>Copied to clipboard!</div>
    {:else}
      <button on:click={async () => await createGroupInvitation()}
        ><strong>Create invitation</strong></button
      >
    {/if}
  </div>
</Popup>
