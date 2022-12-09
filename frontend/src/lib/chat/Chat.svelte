<script lang="ts">
    import { isBlocked } from "../api/socket";
    import { changeGroup, sendMessage,requestMessageLoad} from "../api/socket"
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
    let groupId = "";

    onMount(() => {
        console.log("Chat mounted to the DOM");
       
    });

    afterUpdate(() => {
        if (isLoading) {
            console.log("Chat scrolling up");
            chatBox.scrollTo({ top: chatBox.scrollHeight - chatBoxHeight });
        } else {
            console.log("Chat scrolling down");
            chatBox.scrollTo({ top: chatBox.scrollHeight });
        }
    });

    function sendMessageAction(e: CustomEvent<string>) {
        isLoading = false;
        const message = e.detail;
        sendMessage(message);
    }

    function changeGroupAction(e: CustomEvent<Group>) {
        isLoading = false;
        const {id,name} = e.detail;
        groupId = id;
        groupName = name;
        changeGroup(groupId);
    }

    function parseScroll() {
        isLoading = true;
        if (chatBox.scrollTop == 0 && !$isBlocked) {
            chatBoxHeight = chatBox.scrollHeight;
            requestMessageLoad();
        }
    }

    onDestroy(() => {
        console.log("Chat destroyed");
        
    });
</script>

<div>
    {#if !chatAvailable}
        <div>Connection interrupted, try refreshing page</div>
    {/if}
    <Invitation bind:group_id={groupId} />
    <Groups on:groupSelect={changeGroupAction} />
    <div class="chatbox" bind:this={chatBox} on:scroll={parseScroll}>
        {#if $isBlocked}
            <div>This is the beggining of your chad conversation</div>
        {/if}
        {#key $messages}
            {#each $messages as message}
                <Message {message} />
            {/each}
        {/key}
    </div>
    <Input isDisabled={!chatAvailable} on:message={sendMessageAction} {groupName} />

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
