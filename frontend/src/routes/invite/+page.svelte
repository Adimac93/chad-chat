<script lang="ts">
	import { getGroups, getInvitationID, type Group } from "$lib/api/groups";
	import { onMount } from "svelte";

	let selectedGroup: string = "";
	let id: undefined | string;

	let groups: Array<Group> = [];

	const tryGetInvitationLink = async () => {
		if (!selectedGroup) return;
		
		const res = await getInvitationID(selectedGroup)
		if (!res) return;

		id = res;
		
	}

	async function main() {
		groups = await getGroups();
	}
	onMount(async () =>{
		await main()
	})
</script>

<button on:click={tryGetInvitationLink}>Get invitation</button>
<div></div>
<select bind:value={selectedGroup}>
	{#each groups as group}
		<option value={group.id} >{group.name}</option>
	{/each}
</select>	
{#if id}

	<h2>Copy <a href={`/join/${id}`}>this</a> link to invite your friend to group</h2>
	
{/if}