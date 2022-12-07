<button class="btn" on:click><slot /></button>

<style>
    .btn:hover {
        text-decoration: underline;
        color: #535bf2;
        cursor: pointer;
    }
    .btn {
        font-weight: 500;
        color: #646cff;
        text-decoration: inherit;
    }
</style>
