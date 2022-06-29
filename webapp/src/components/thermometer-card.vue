<template>
  <div>
    <h2>{{ name }}</h2>
    <div ref="graph"></div>
    <p>température: {{ thermometer?.last_measurement }}</p>
    <p>température cible: {{ thermometer?.target_temperature }}</p>
  </div>
</template>

<style lang="scss">
@import "../assets/base.scss";
</style>

<script lang="ts">
import { defineComponent } from "vue";
import Plotly from "plotly.js-dist-min";
import type { thermometer } from "../thermometer";

export default defineComponent({
  props: {
    name: String,
  },
  data() {
    return {
      thermometer: null as thermometer | null,
      thermometer_history: [] as [string, thermometer][],
    };
  },

  created() {
    let poll_status = async () => {
      const response = await fetch(`/rest-api/thermometer-status/${this.name}`);

      this.thermometer = await response.json();
    };

    let poll_history = async () => {
      const response = await fetch(
        `/rest-api/thermometer-history/${this.name}`
      );

      this.thermometer_history = await response.json();
    };

    poll_history();
    poll_status();
    setInterval(poll_history, 3000);
    setInterval(poll_status, 3000);
  },

  mounted() {
    let trace1 = {
      x: [1, 2, 3, 4],

      y: [10, 15, 13, 17],
    };

    let data = trace1;
    Plotly.newPlot(this.$refs.graph, [data]);
  },
});
</script>
