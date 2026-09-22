<template>
  <div class="main">{{ greeting }}</div>
</template>

<script>
import helper from "./helper.vue";
import fs from "fs";

export default {
  computed: {
    greeting() {
      return helper(fs.sep);
    },
  },
};
</script>

<style scoped>
.main {
  color: black;
}
</style>
