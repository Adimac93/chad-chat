<script lang="ts">
  import { Socket } from "./socket";
  import Groups from "./Groups.svelte";
  import { Action } from "./socket";
  import { onDestroy, onMount } from "svelte";
  import Message from "./Message.svelte";
  import Input from "./Input.svelte";
  import { beforeUpdate, afterUpdate } from "svelte";
  import { messages } from "../stores";
  import Create from "./Create.svelte";

  let groupName = "";
  let chatBox: HTMLElement;
  let chatBoxHeight: number;
  let isLoading = false;
  const socket = new Socket();

  onMount(() => {});

  afterUpdate(() => {
    if (isLoading) {
      console.log("scroll up");
      chatBox.scrollTo({ top: chatBox.scrollHeight - chatBoxHeight });
      socket.isBlocked = socket.isBlocked;
    } else {
      console.log("scroll down");
      chatBox.scrollTo({ top: chatBox.scrollHeight });
    }
  });

  function sendMessage(e: CustomEvent<string>) {
    isLoading = false;
    const message = e.detail;
    socket.sendMessage(message);
  }

  function changeGroup(e: CustomEvent<Group>) {
    isLoading = false;
    const group = e.detail;
    groupName = group.name;
    socket.changeGroup(group.id);
  }

  function parseScroll() {
    isLoading = true;
    if (chatBox.scrollTop == 0) {
      chatBoxHeight = chatBox.scrollHeight;
      socket.requestMessageLoad();
    }
  }

  onDestroy(() => {
    socket.webSocket.close();
  });
</script>

<div>
  <Groups on:groupSelect={changeGroup} />
  <div class="chatbox" bind:this={chatBox} on:scroll={parseScroll}>
    {#if socket.isBlocked}
      <div>This is the beggining of your chad conversation</div>
    {/if}
    {#key $messages}
      {#each $messages as message}
        <Message {message} />
      {/each}
    {/key}
  </div>
  <Input on:message={sendMessage} {groupName} />

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
</div>
