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

	let chatMessages: Array<Message> = [];

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

	interface Message {
		sender: string,
		sat: number,
		content: string,
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
			const msg = JSON.parse(e.data);
			console.log(msg);
			const keys = Object.keys(msg);
			
			if (keys[0] == "LoadMessages") {
				chatMessages = []
				const messages = (msg.LoadMessages as Array<Message>)
				messages.forEach((m)=>{
					chatMessages.push(m);
				})
				chatMessages = chatMessages;
			} else if (keys[0] == "Message") {
				const message = (msg.Message as Message);
				chatMessages.push(message);
				chatMessages = chatMessages;
			} else {
				console.log("error");
			}
		};
		return websocket;
	}
</script>

<h1>Chat</h1>
<div class="justify-center items-center h-1/2 w-auto">
		<select bind:value={selectedGroup} on:change={changeChat} class="block px-10 rounded-mdsele">
			<option disabled selected>Select chat, chad</option>
			{#each groups as group}
				<option value={group}>{group.name}</option>
			{/each}
		</select>
		<div class="block w-1/3 h-2/3 box-border border-4 rounded-lg rounded-b-none overflow-y-scroll max-h-96 scroll">
			{#each chatMessages as message}
				<div class="block">{new Date(message.sat).toLocaleTimeString()} {message.sender}: {message.content}</div>
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
