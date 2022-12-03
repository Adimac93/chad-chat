<script lang="ts">
  import { isAuthorized, messages } from "../stores";

  async function logout() {
    isAuthorized.set(false);
    messages.set([]);
    const res = await fetch("/api/auth/logout", { method: "POST" });
    if (res.ok) {
      console.log("Logout");
    }
  }
</script>

<button on:click={logout}>Logout</button>
