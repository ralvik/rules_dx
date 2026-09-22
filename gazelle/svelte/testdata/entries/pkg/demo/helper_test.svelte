<script>
  import helper from "./helper.svelte";

  let greeting = $derived(helper("x"));
</script>

<div class="helper-test">{greeting}</div>

<style>
  .helper-test {
    color: black;
  }
</style>
