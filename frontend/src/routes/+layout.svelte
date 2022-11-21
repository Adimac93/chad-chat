<script lang="ts">
	import '../app.sass';
</script>

<div class="min-h-full">
	<nav class="navbar border-b bg-gray-200">
		<div class="flex-1">
			<a href="/" class="text-xl normal-case">Home</a>
			<a href="/login" class="hover text-lg normal-case text-orange-500 hover:text-orange-400"
				>Login</a
			>
			<a href="/register" class="hover text-lg normal-case text-orange-500 hover:text-orange-400"
				>Register</a
			>
			<a href="/chat" class="hover text-lg normal-case text-orange-500 hover:text-orange-400"
				>Chat</a
			>
		</div>
	</nav>
	<slot />
</div>
