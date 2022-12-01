<script lang="ts">
  import { Socket } from "./socket";
  import Groups from "./groups/Groups.svelte";

  import { onDestroy, onMount } from "svelte";
  import Message from "./Message.svelte";
  import Input from "./Input.svelte";
  import { beforeUpdate, afterUpdate } from "svelte";
  import { messages } from "../stores";
  import Invitation from "./groups/invitation/Invitation.svelte";

  let groupName = "";
  let chatBox: HTMLElement;
  let chatBoxHeight: number;
  let isLoading = false;
  let chatAvailable = true;

  let socket = new Socket();

  let groupId = "";

  socket.webSocket.onclose = (e) => {
    chatAvailable = false;
    console.log("Reconnecting");
    setInterval(() => {
      socket.connect();
    }, 10000);
  };

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
    groupId = group.id;
    groupName = group.name;
    socket.changeGroup(groupId);
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
    socket = null;
  });
</script>

<div>
  {#if !chatAvailable}
    <div>Connection interrupted, try refreshing page</div>
  {/if}
  <Invitation bind:group_id={groupId} />
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
