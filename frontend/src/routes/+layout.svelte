<script lang="ts">
	import '../app.sass';
</script>

<div class="min-h-full">
	<nav class="navbar bg-gray-200 border-b">
		<div class="flex-1">
			<a href="/" class="btn btn-ghost normal-case text-xl">Home</a>
			<a href="/login">Login</a>
			<a href="/register">Register</a>
			<a href="/chat">Chat</a>
		</div>
	</nav>
	<slot />
</div>
