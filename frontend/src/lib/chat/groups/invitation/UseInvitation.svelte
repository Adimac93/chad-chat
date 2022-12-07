<script lang="ts">
    import { afterUpdate, createEventDispatcher } from "svelte";
    import { request } from "../../../api/fetch";

    let name;
    let members;

    let code = "";
    let error = "";

    let isCorrect = false;

    const dispatch = createEventDispatcher<{ join }>();

    async function getGroupInfo() {
        const res = await request("/api/groups/invitations/info", {
            method: "POST",
            body: { code },
        });
        if (res.ok) {
            isCorrect = true;
            name = res.data.name as string;
            members = res.data.members as number;
        } else {
            error = res.data.error_info;
            setTimeout(() => {
                error = "";
            }, 5000);
            code = "";
        }
    }
    async function joinGroup() {
        let res = await request(`/api/groups/invitations/join`, {
            method: "POST",
            body: { code },
        });

        if (res.ok) {
            dispatch("join");
        } else {
            error = res.data.error_info;
            setTimeout(() => {
                error = "";
            }, 5000);
            code = "";
        }
    }

    $: checkCode(code);

    const checkCode = (text: string) => {
        code = text.replaceAll(" ", "");
        if (code.length == 10) {
            getGroupInfo();
        }
    };
</script>

<div class="card">
    {#if isCorrect}
        <strong>Group: {name}</strong>
        <div>Members: {members}</div>
        <button on:click={joinGroup}>Join</button>
    {/if}
    <div>Enter group join code:</div>
    <input disabled={isCorrect} bind:value={code} type="text" />
    <div>{error}</div>
</div>

<style>
    div {
        margin: 20px;
    }
    input {
        text-align: center;
    }
</style>
