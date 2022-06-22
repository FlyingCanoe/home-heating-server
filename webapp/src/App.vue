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
import Plotly from "plotly.js-dist-min";

export default defineComponent({
  data() {
    return {
      thermometer_list: [] as [string, thermometer][],
      thermometer_history_list: [] as [string, thermometer[]][],
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

    let poll_history = async () => {
      this.thermometer_list.forEach(async (thermometer) => {
        let name = thermometer[0];
        const response = await fetch(`/rest-api/thermometer-history/${name}`);
        const thermometer_history_list: [string, thermometer[]][] =
          await response.json();
        this.thermometer_history_list = thermometer_history_list;
      });
    };
    poll_history();
    setInterval(poll_history, 3000);
  },
});

let trace1 = {
  x: [1, 2, 3, 4],

  y: [10, 15, 13, 17],
};

let data = trace1;

Plotly.newPlot("myDiv", [data]);
</script>
