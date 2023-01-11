<script lang="ts">
    import { isAuthorized } from "../stores";

    let email = "";
    let passwordA = "";
    let passwordB = "";
    let message = "";
    let username = "";

    async function register_user() {
        let res = await fetch(`api/auth/register`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
                email,
                password: passwordA,
                username
            }),
        });
        if (res.ok) {
            isAuthorized.set(true);
        } else {
            const json = await res.json();
            message = json.error_info;
            setTimeout(() => {
                message = "";
            }, 5000);
        }
    }
</script>

<form>
    <input bind:value={email} placeholder="email" type="email" />
    <input bind:value={username} placeholder="username" type="text" />
    <input bind:value={passwordA} placeholder="password" type="password" />
    <input bind:value={passwordB} placeholder="repeat password" type="password" />
    <div>{message}</div>
    <button
        on:click|preventDefault={register_user}
        disabled={!email || !passwordA || !passwordB || passwordA != passwordB}>Register</button
    >
</form>

<style>
    input {
        display: block;
        margin: auto;
        margin-bottom: 0.5em;
        align-items: center;
        text-align: center;
    }
</style>
