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
	let selectedGroup: Group = {id:"",name:""};
	let groups: Array<Group> = [];
	let websocket: WebSocket;

	let messages: Array<string> = [];

	onMount(async () => {
		groups = await getGroups();
		if (groups.length > 0) {
			let groupID = localStorage.getItem("group");
			let group = groups.find(({id,name})=>id == groupID);
			
			selectedGroup = group? group : groups[0];
			await changeChat();
		}
		
			
		
	});

	async function changeChat() {
		if (websocket) websocket.close();
		websocket = initWebsocket();
		localStorage.setItem("group",selectedGroup.id)
		textbox = '';
	}

	function onKeyDown(e: KeyboardEvent) {
		if (e.key == 'Enter' && input) {
			websocket.send(JSON.stringify({SendMessage: {content: input}}));
			input = '';
		}
	}

	function initWebsocket() {
		const websocket = new WebSocket(`ws://${variables.api}/chat/websocket`);

		websocket.onopen = () => {
			console.log(`connection opened`);
			console.log(selectedGroup);
			websocket.send(JSON.stringify({ChangeGroup: {group_id: selectedGroup.id}}));
		};

		websocket.onclose = () => {
			console.log('connection closed');
		};

		websocket.onmessage = (e) => {
			console.log('received message: ' + e.data);
			// textbox += e.data + '\r\n';
			messages.push(e.data);
			messages = messages;
			
		};
		return websocket;
	}
</script>

<h1>Chat</h1>
<div class="justify-center items-center h-64">
		<select bind:value={selectedGroup} on:change={changeChat} class="block px-10 rounded-mdsele">
			<option disabled selected>Select chat, chad</option>
			{#each groups as group}
				<option value={group}>{group.name}</option>
			{/each}
		</select>
		<div class="block w-1/3 h-2/3 min-h- box-border border-4 rounded-lg rounded-b-none overflow-y-scroll max-h-96 scroll">
			{#each messages as message}
				<div class="block">{message}</div>
			{/each}
		</div>
		<input
			on:keydown={onKeyDown}
			bind:value={input}
			class="block w-1/3 box-border border-4 rounded-b-md border-t-0"
			type="text"
			placeholder={`send message to: ${selectedGroup.name}`}
		/>
</div>
