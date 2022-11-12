<script lang="ts">
	import { goto } from "$app/navigation";
	import { createNewGroup } from "$lib/api/groups";

    let groupName = "";
    let errorMessage = ""
    const createGroup = async () => {
        let isOk = await createNewGroup(groupName);
        if (isOk) {
            //await goto("/chat")
            console.log("OK")
        } else {
            errorMessage = `Failed to create group ${groupName}`;
        }
    }
    
</script>

<input bind:value={groupName} type="text">
<button on:click={createGroup}>Create group</button>
{#if errorMessage}
    <h3>{errorMessage}</h3>
{/if}