<h1>Chat</h1>
<select bind:value={selectedGroup} on:change={changeSocket}>
    {#each groups as group}
        <option value={group.id}>{group.name}</option>
    {/each}
</select>
<textarea bind:value={textbox} id="chat" style="display:block; width:600px; height:400px; box-sizing: border-box" cols="30" rows="10" disabled="{true}" ></textarea>
<input on:keydown={onKeyDown} bind:value={input} style="display:block; width:600px; box-sizing: border-box" type="text" placeholder="chat">


<script lang="ts">
    import {variables} from "$lib/variables";
	import { onMount } from "svelte";

    interface Group {
        id: string,
        name: string
    }
    let textbox = "";
    let input = "";
    let selectedGroup = "";
    let groups: Array<Group> = [];
    let websocket: WebSocket;

    onMount(async () =>{
        await getGroups();
        websocket = initWebsocket();
    })
    

    function changeSocket() {
        websocket.close();
        websocket = initWebsocket()
    }

    function onKeyDown(e: KeyboardEvent) {
        if (e.key == "Enter") {
            websocket.send(input);
            input = "";
        }
    }

    async function getGroups() {
        let res = await fetch(`http://${variables.basePath}/chat/groups`, { method: "GET", mode: "cors", credentials: "include" });
        let json = await res.json();
        groups = json.groups;
        console.log(groups);
    };

    function initWebsocket() {
        const websocket = new WebSocket(`ws://${variables.basePath}/chat/websocket`);

        websocket.onopen = () => {
            console.log(`connection opened`);
            websocket.send(selectedGroup);

        }

        websocket.onclose = () => {
            console.log("connection closed");
        }

        websocket.onmessage = (e) => {
            console.log("received message: " + e.data);
            textbox += e.data + "\r\n";
        }
        return websocket
    }


</script>