<script lang="ts">
	import { variables } from '$lib/variables';

	let login = '';
	let password = '';
	let repeated_password = '';
	let message: number;

	async function login_user() {
		if (password != repeated_password) return;

		let res = await fetch(`http://${variables.api}/auth/register`, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({
				login,
				password
			})
		});
		message = (await res.json()).error_info;
	}
</script>

<h1>Register</h1>
<form class="flex-box text-center">
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
	<label class="m-2 block"
		>Repeat password<br /><input
			bind:value={repeated_password}
			placeholder="repeat password"
			type="password"
			class="rounded-md border-4"
		/></label
	>
	<button
		class="content m-6 rounded-lg border-4 px-4 hover:bg-slate-100"
		on:click|preventDefault={login_user}>Register</button
	>
</form>
