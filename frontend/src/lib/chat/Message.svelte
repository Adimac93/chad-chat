<script lang="ts">
  export let message: MessageModel;
  const date = new Date(message.sat * 1000).toLocaleTimeString();
  const sender = message.nickname;

  const parts: Array<Part> = [];

  for (let word of message.content.split(" ")) {
    const isUrl =
      /(http:\/\/www\.|https:\/\/www\.|http:\/\/|https:\/\/)?[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,5}(:[0-9]{1,5})?(\/.*)?/gm.test(
        word
      );
    parts.push({ isUrl, payload: word + " " });
  }

  interface Part {
    payload: string;
    isUrl: boolean;
  }
</script>

<div class="message">
  <strong class="sender">{sender}</strong>
  <i class="date">{date}</i>
  <div class="content">
    {#each parts as part}
      {#if part.isUrl}
        <a class="part" href={part.payload}>{part.payload}</a>
      {:else}
        <div class="part">{part.payload}</div>
      {/if}
    {/each}
  </div>
</div>

<style>
  .message {
    text-align: left;
  }
  .message:hover {
    filter: brightness(85%);
  }
  .sender {
    display: inline;
    text-align: left;
  }
  .date {
    display: inline;
    text-align: right;
  }
  .content {
    text-align: left;
  }

  .part {
    display: inline;
  }
</style>
