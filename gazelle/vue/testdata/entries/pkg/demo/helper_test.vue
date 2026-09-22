<template>
  <div class="helper-test">{{ greeting }}</div>
</template>

<script>
import helper from "./helper.vue";

export default {
  computed: {
    greeting() {
      return helper("x");
    },
  },
};
</script>

<style scoped>
.helper-test {
  color: black;
}
</style>
