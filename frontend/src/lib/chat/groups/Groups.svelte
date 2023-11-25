<script lang="ts">
    import { createEventDispatcher, onMount } from "svelte";
    import { getGroups } from "../../api/groups";
    import CreateGroup from "./CreateGroup.svelte";

    const dispatch = createEventDispatcher<{ groupSelect: Group }>();

    let groups: Array<Group> = [];
    let selected: Group;

    async function fetchGroups() {
        groups = await getGroups();
    }
    onMount(async () => {
        await fetchGroups();
    });

    function groupSelect() {
        console.log(`Selected group ${selected.name}`);

        dispatch("groupSelect", selected);
    }
</script>

<select bind:value={selected} on:change={groupSelect}>
    <option selected disabled hidden>Select chat, chad</option>
    {#each groups as group}
        <option value={group}>{group.name}</option>
    {/each}
</select>
<CreateGroup on:groupCreate={async () => await fetchGroups()} />
