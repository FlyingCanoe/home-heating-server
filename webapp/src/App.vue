<template>
  <div class="card-group">
    <div
      class="card"
      v-for="thermometer in thermometer_list"
      v-bind:key="thermometer"
    >
      <thermometerCard :name="thermometer" />
    </div>
  </div>
</template>

<style lang="scss">
@import "./assets/base.scss";
</style>

<script lang="ts">
import { defineComponent } from "vue";
import type { thermometer } from "./thermometer";
import thermometerCard from "./components/thermometer-card.vue";

export default defineComponent({
  data() {
    return {
      thermometer_list: [] as string[],
      thermometer_history_list: [] as [string, thermometer][],
    };
  },

  created() {
    let poll_api = async () => {
      const response = await fetch("/rest-api/thermometer-list");
      const thermometer_list: string[] = await response.json();
      this.thermometer_list = thermometer_list;

      console.log(thermometer_list);
    };
    poll_api();
    setInterval(poll_api, 3000);
  },

  components: {
    thermometerCard,
  },
});
</script>
