<script lang="ts">
	import { variables } from '$lib/variables';
	import { getGroups } from '$lib/api/groups';
	import { onMount } from 'svelte';

	interface Group {
		id: string;
		name: string;
	}
	let textbox = '';
	let input = '';
	let selectedGroup = '';
	let groups: Array<Group> = [];
	let websocket: WebSocket;

	onMount(async () => {
		groups = await getGroups();
	});

	function changeChat() {
		if (websocket) websocket.close();
		websocket = initWebsocket();
		textbox = '';
	}

	function onKeyDown(e: KeyboardEvent) {
		if (e.key == 'Enter') {
			websocket.send(input);
			input = '';
		}
	}

	function initWebsocket() {
		const websocket = new WebSocket(`ws://${variables.api}/chat/websocket`);

		websocket.onopen = () => {
			console.log(`connection opened`);
			console.log(selectedGroup);
			websocket.send(selectedGroup);
		};

		websocket.onclose = () => {
			console.log('connection closed');
		};

		websocket.onmessage = (e) => {
			console.log('received message: ' + e.data);
			textbox += e.data + '\r\n';
		};
		return websocket;
	}
</script>

<h1>Chat</h1>
<select bind:value={selectedGroup} on:change={changeChat}>
	{#each groups as group}
		<option value={group.id}>{group.name}</option>
	{/each}
</select>
<textarea
	bind:value={textbox}
	id="chat"
	style="display:block; width:600px; height:400px; box-sizing: border-box"
	cols="30"
	rows="10"
	disabled={true}
/>
<input
	on:keydown={onKeyDown}
	bind:value={input}
	style="display:block; width:600px; box-sizing: border-box"
	type="text"
	placeholder="chat"
/>
