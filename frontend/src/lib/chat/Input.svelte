<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let groupName: string;

  const dispatch = createEventDispatcher<{ message: string }>();

  let content: string = "";
  function onEnterKeyDown(e: KeyboardEvent) {
    if (e.key == "Enter" && content.trim()) {
      dispatch("message", content);
      content = "";
    }
  }
</script>

<input
  on:keydown={onEnterKeyDown}
  bind:value={content}
  type="text"
  placeholder={`Message ${groupName}`}
/>

<style>
  input {
    min-width: 25em;
  }
</style>
