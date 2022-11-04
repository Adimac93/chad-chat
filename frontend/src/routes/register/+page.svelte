<h1>Register</h1>
<form>
    <label>Login<input bind:value={login} placeholder="login" type="text"></label>
    <label>Password<input bind:value={password} placeholder="password" type="password"></label>
    <label>Repeat password<input bind:value={repeated_password} placeholder="password" type="password"></label>
    <button on:click|preventDefault={login_user}>Submit form</button>
</form>
<div>Status: {message}</div>

<script lang="ts">
    let login = "";
    let password = "";
    let repeated_password = "";
    let message: number;

    async function login_user() {
        if (password != repeated_password) return

        let res = await fetch(`http://127.0.0.1:3000/auth/register`,{
            method: "POST",
            headers: {"Content-Type":"application/json"},
            body : JSON.stringify({
                login,
                password,
            })
        })
        message = res.status;
    }
    
</script>