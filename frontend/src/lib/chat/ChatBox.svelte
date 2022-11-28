<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import Input from "./Input.svelte";
  import Message from "./Message.svelte";

  export let messages: Array<MessageModel>;
  export let groupName: string;

  let box: HTMLElement;
  let yScroll = 0;
  let scrollPercent = 0;
  let loading = false;
  const dispatch = createEventDispatcher<{
    messagesRequest;
  }>();

  function parseScroll() {
    if (box.scrollTop == 0) {
      let height = box.scrollHeight;
      console.log(height);
      setTimeout(() => {
        dispatch("messagesRequest");

        loading = true;
        setTimeout(() => {
          loading = false;
          box.scrollTo({ top: box.scrollHeight - height });
        }, 100);
      }, 300);
    }
  }
</script>

<div class="chatbox" bind:this={box} on:scroll={parseScroll}>
  <!-- {#if loading === false}
    <div bind:this={loadElement}>Loading...</div>
  {/if} -->
  {#key messages}
    {#each messages as message}
      <Message {message} />
    {/each}
  {/key}
</div>
<Input on:message {groupName} />

<style>
  .chatbox {
    min-width: 50em;
    padding: 1em;
    min-height: 20em;
    max-height: 100px;
    border: 0.1em solid #646cff;
    border-radius: 1em;
    overflow-y: scroll;
  }
</style>
