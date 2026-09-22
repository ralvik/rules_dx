<script>
  import helper from "./helper.svelte";
  import fs from "fs";

  let greeting = $derived(helper(fs.sep));
</script>

<div class="main">{greeting}</div>

<style>
  .main {
    color: black;
  }
</style>
