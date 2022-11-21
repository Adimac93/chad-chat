<script lang="ts">
	import { goto } from '$app/navigation';
	import { variables } from '$lib/variables';

	let login = '';
	let password = '';
	let message: number;

	async function login_user() {
		console.log(`${login} - ${password}`);
		let res = await fetch(`http://${variables.api}/auth/login`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				login,
				password
			}),
			mode: 'cors',
			credentials: 'include'
		});
		if (res.ok) {
			goto(`/chat`);
		}
	}
</script>

<form class="text-center">
	<label class="m-2 block"
		>Login<br /><input
			bind:value={login}
			placeholder="login"
			type="text"
			class="rounded-md border-4"
		/></label
	>
	<label class="m-2 block"
		>Password<br /><input
			bind:value={password}
			placeholder="password"
			type="password"
			class="rounded-md border-4"
		/></label
	>
	<button
		class="content m-6 rounded-lg border-4 px-4 hover:bg-slate-100"
		on:click|preventDefault={login_user}>Login</button
	>
</form>
