<template>
  <div class="card-group">
    <div
      class="card"
      v-for="thermometer in thermometer_list_ext"
      v-bind:key="thermometer[0]"
    >
      <h2>{{ thermometer[0] }}</h2>
      <p>température: {{ thermometer[1].last_measurement }}</p>
      <p>température cible: {{ thermometer[1].target_temperature }}</p>
      <p>delta: {{ thermometer[2] }}</p>
    </div>
  </div>
</template>

<style lang="scss">
@import "./assets/base.scss";
</style>

<script lang="ts">
import { defineComponent } from "vue";
import type { thermometer } from "./thermometer";

export default defineComponent({
  data() {
    return {
      thermometer_list: [] as [string, thermometer][],
    };
  },

  computed: {
    thermometer_list_ext() {
      let thermometer_list_ext = [] as [string, thermometer, number | null][];
      this.thermometer_list.forEach((value) => {
        let name = value[0];
        let thermometer = value[1];

        let delta = null;
        if (
          thermometer.target_temperature != null &&
          thermometer.last_measurement != null
        ) {
          delta = thermometer.target_temperature - thermometer.last_measurement;
        }

        thermometer_list_ext.push([name, thermometer, delta]);
      });
      return thermometer_list_ext;
    },
  },

  created() {
    let poll_api = async () => {
      const response = await fetch("/rest-api/thermometer-status");
      const thermometer_list: [string, thermometer][] = await response.json();
      this.thermometer_list = thermometer_list;
    };
    poll_api();
    setInterval(poll_api, 3000);
  },
});
</script>
