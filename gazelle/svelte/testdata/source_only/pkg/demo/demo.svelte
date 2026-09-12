<script>
  import helper from "./helper.svelte";
  import fs from "fs";

  let greeting = $derived(helper(fs.sep));
</script>

<div class="demo">{greeting}</div>

<style>
  .demo {
    color: black;
  }
</style>
