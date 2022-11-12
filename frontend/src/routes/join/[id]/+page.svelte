<script lang="ts">
	import { joinGroupById } from '$lib/api/groups';
    import type {PageData} from './$types'
    export let data: PageData;

    let joined = false;
    const joinGroup = async () => {
       joined = await joinGroupById(data.id)
    }

</script>
{#if data.info}
    {#if joined}
        <h1>Successfully joined {data.info.name}</h1>
    {:else}
        <h1>Group: {data.info.name} with {data.info.members} members</h1>
        <button on:click={joinGroup}>Join</button>
    {/if}
{/if}