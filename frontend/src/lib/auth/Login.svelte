<script lang="ts">
  import { isAuthorized } from "../stores";
  import TextButton from "../TextButton.svelte";

  let login = "";
  let password = "";

  let buttonText = "Forgor your password, worry not, just click this button";
  let message = "";

  async function login_user() {
    let res = await fetch(`api/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        login,
        password,
      }),
    });
    if (res.ok) {
      isAuthorized.set(true);
    } else {
      message = "True chads remember credensials...";
      setTimeout(() => {
        message = "";
      }, 5000);
    }
  }
</script>

<form>
  <input bind:value={login} placeholder="login" type="text" />
  <input bind:value={password} placeholder="password" type="password" />
  <div>{message}</div>
  <button on:click|preventDefault={login_user} disabled={!login || !password}
    >Login</button
  >
</form>
<TextButton on:click={() => (buttonText = "Just kidding")}
  >{buttonText}</TextButton
>

<style>
  input {
    display: block;
    margin: auto;
    margin-bottom: 0.5em;
    align-items: center;
    text-align: center;
  }
</style>
