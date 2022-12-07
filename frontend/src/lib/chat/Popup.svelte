<script lang="ts">
    import { createEventDispatcher } from "svelte";

    export let isActive;
    const dispatch = createEventDispatcher<{ close }>();
    const closePopup = () => {
        isActive = false;
        dispatch("close");
    };
</script>

{#if isActive}
    <div class="container">
        <div class="popup">
            <br />
            <button id="close-button" on:click={closePopup}>
                <svg id="icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 512"
                    ><path
                        d="M310.6 150.6c12.5-12.5 12.5-32.8 0-45.3s-32.8-12.5-45.3 0L160 210.7 54.6 105.4c-12.5-12.5-32.8-12.5-45.3 0s-12.5 32.8 0 45.3L114.7 256 9.4 361.4c-12.5 12.5-12.5 32.8 0 45.3s32.8 12.5 45.3 0L160 301.3 265.4 406.6c12.5 12.5 32.8 12.5 45.3 0s12.5-32.8 0-45.3L205.3 256 310.6 150.6z"
                    /></svg
                >
            </button>
            <slot />
        </div>
    </div>
{/if}
<button disabled={isActive} on:click={() => (isActive = !isActive)}>Invitations</button>

<style>
    .popup {
        border: 0.1em solid #f9f9f9;
        background-color: #242424;
        border-radius: 8px;
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        text-align: center;
        visibility: visible;
    }
    .container {
        top: 50%;
        left: 50%;
        width: 100%;
        height: 100%;
        transform: translate(-50%, -50%);
        backdrop-filter: blur(3px);
        position: absolute;
        visibility: visible;
        display: block;
    }

    #close-button {
        padding: 11.5px;
        width: 50px;
        height: 50px;
    }
    #icon {
        width: 25px;
        height: 25px;
    }
    @media (prefers-color-scheme: light) {
        #icon {
            fill: #213547;
        }

        .popup {
            border: 0.1em solid #646cff;
            background-color: white;
        }
    }
</style>
