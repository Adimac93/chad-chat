<script lang="ts">
    import { request } from "../../../api/fetch";
    export let group_id: string;

    const expirationChoice = ["30 minutes", "1 hour", "6 hours", "12 hours", "1 day", "Never"];
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
    let usage_index;
    let code = "";
    let error = "";

    async function copyToClipboard(text: string) {
        await navigator.clipboard.writeText(text);
    }

    async function createGroupInvitation() {
        console.debug({ group_id, expiration_index, usage_index });
        const res = await request("/api/groups/invitations/create", {
            method: "POST",
            body: {
                group_id,
                expiration_index:
                expiration_index == expirationChoice.length - 1 ? null : expiration_index,
                usage_index: 
                usage_index == usesChoices.length - 1 ? null : usage_index,
            },
        });
        if (res.ok) {
            code = res.data.code as string;
            await copyToClipboard(code);
        } else {
            error = res.data.error_info || "Group not selected";
        }
    }
</script>

<div class="card">
    <div>Expire after</div>
    <select bind:value={expiration_index} disabled={!!code}>
        {#each expirationChoice as exp, i}
            <option value={i}>{exp}</option>
        {/each}
    </select>

    <div>Max number of uses</div>
    <select bind:value={usage_index} disabled={!!code}>
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
    <div>{error}</div>
</div>
