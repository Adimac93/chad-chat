<script lang="ts">
    import { isAuthorized } from "../stores";

    let login = "";
    let passwordA = "";
    let passwordB = "";
    let message = "";
    let nickname = "";

    async function register_user() {
        let res = await fetch(`api/auth/register`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
                login,
                password: passwordA,
                nickname,
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
    <input bind:value={login} placeholder="login" type="text" />
    <input bind:value={nickname} placeholder="nickname" type="text" />
    <input bind:value={passwordA} placeholder="password" type="password" />
    <input bind:value={passwordB} placeholder="repeat password" type="password" />
    <div>{message}</div>
    <button
        on:click|preventDefault={register_user}
        disabled={!login || !passwordA || !passwordB || passwordA != passwordB}>Register</button
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
