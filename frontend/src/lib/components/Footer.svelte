<script lang="ts">
  const days = Math.round(
    (new Date().getTime() - new Date("2022-11-02").getTime()) / 84_600_000
  );
  // what? new Date(year, monthIndex ???, day)
</script>

<footer>
  <i>chad-chat pre-alpha 0.0.1v optimized for desktop</i>
  <br />
  Made with
  <a
    class="logo"
    href="https://www.rust-lang.org/"
    rel="noreferrer"
    target="_blank">🦀</a
  >
  by
  <a href="https://github.com/Adimac93" rel="noreferrer" target="_blank"
    >@Adimac93</a
  >
  ,
  <a href="https://github.com/HubertK05" rel="noreferrer" target="_blank"
    >@HubertK05</a
  >
  in {days}
  <div class="days"><span>days</span></div>
</footer>

<style>
  footer {
    margin-top: 20px;
  }
  i {
    color: gray;
  }
  a:hover {
    filter: drop-shadow(0 0 1em #646cffaa);
  }
  .logo:hover {
    filter: drop-shadow(0 0 1em salmon);
    cursor: pointer;
  }
  .logo {
    display: inline;
    text-shadow: 0 0 5px #f96f18;
  }
  .days {
    display: inline;
  }
  .days:hover:before {
    display: inline;
    content: "days and the number will grow like a 🌴";
    text-shadow: 0 0 5px #29d946;
  }
  .days:hover span {
    display: none;
  }
</style>
