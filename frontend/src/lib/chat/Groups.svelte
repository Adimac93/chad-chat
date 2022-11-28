<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { getGroups } from "../api/groups";
  import Create from "./Create.svelte";

  const dispatch = createEventDispatcher<{ groupSelect: Group }>();

  let groups: Array<Group> = [];
  let selected: Group;

  async function fetchGroups() {
    groups = await getGroups();
  }
  onMount(async () => {
    await fetchGroups();
    if (groups.length > 0) {
      let savedGroupID = localStorage.getItem("group");
      if (savedGroupID) {
        let group = groups.find(({ id }) => id == savedGroupID);
        selected = group ? group : groups[0];
        groupSelect();
      }

      localStorage.setItem("group", selected.id);
    }
  });

  function groupSelect() {
    dispatch("groupSelect", selected);
  }
</script>

<select bind:value={selected} on:change={groupSelect}>
  <option selected disabled hidden>Select chat, chad</option>
  {#each groups as group}
    <option value={group}>{group.name}</option>
  {/each}
</select>
<Create on:groupCreate={async () => await fetchGroups()} />

<style>
  select {
    border-radius: 8px;
    border: 1px solid transparent;
    padding: 0.6em 1.2em;
    margin: 0.5em;
    font-size: 1em;
    font-weight: 500;
    font-family: inherit;
    background-color: #1a1a1a;
    cursor: pointer;
    transition: border-color 0.25s;
  }

  option {
    border-radius: 8px;
    border: 1px solid transparent;
    padding: 0.6em 1.2em;
    margin: 0.5em;
    font-size: 1em;
    font-weight: 500;
    font-family: inherit;
    background-color: #1a1a1a;
    cursor: pointer;
    transition: border-color 0.25s;
  }
</style>
