<script lang="ts">
  export let message: MessageModel;
  let date = new Date(message.sat * 1000).toLocaleTimeString();
  let sender = message.sender;
  let content = message.content;
</script>

<div class="message">
  <strong class="sender">{sender}</strong>
  <i class="date">{date}</i>
  <div class="content">{content}</div>
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
</style>
