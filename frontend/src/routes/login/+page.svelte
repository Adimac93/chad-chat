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
			credentials: 'include'
		});
		message = res.status;
		if (res.ok) {
			goto(`/chat`);
		}
	}
</script>

<h1>Login</h1>
<form>
	<label>Login<input bind:value={login} placeholder="login" type="text" /></label>
	<label>Password<input bind:value={password} placeholder="password" type="password" /></label>
	<button on:click|preventDefault={login_user}>Login</button>
</form>
<div>Status: {message}</div>
