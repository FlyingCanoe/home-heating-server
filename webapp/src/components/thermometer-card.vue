<template>
  <h2>{{ name }}</h2>
  <uPlotVue :data="data" :options="plot_option" />
</template>

<style lang="scss">
@import "../assets/base.scss";
</style>

<script lang="ts">
import { defineComponent } from "vue";
import UplotVue from "uplot-vue";

import "uplot/dist/uPlot.min.css";

import type uPlot from "uplot";
import type { thermometer } from "../thermometer";

export default defineComponent({
  props: {
    name: String,
  },
  data() {
    return {
      thermometer: null as thermometer | null,
      thermometer_history: [] as [number, thermometer][],
      data: [[], [], []] as uPlot.AlignedData,
      plot_option: {
        width: 500,
        height: 300,
        series: [
          {
            label: "heurs",
            value: (_, timestamp) => {
              let date = new Date(timestamp * 1000);
              return `${date.getHours()}:${date.getMinutes()}`;
            },
          },
          {
            label: "température",
            stroke: "red",
            fill: "rgba(255,0,0,0.1)",
          },
          {
            label: "température cible",
            stroke: "blue",
          },
        ],
      } as uPlot.Options,
    };
  },
  components: { uPlotVue: UplotVue },
  created() {
    let poll_status = async () => {
      const response = await fetch(`/rest-api/thermometer-status/${this.name}`);

      this.thermometer = await response.json();
    };

    let poll_history = async () => {
      const response = await fetch(
        `/rest-api/thermometer-history/${this.name}`
      );

      let raw_thermometer_history: [number, thermometer][] =
        await response.json();

      let x = [] as number[];
      let temperature = [] as number[];
      let cible = [] as number[];

      raw_thermometer_history.forEach(([date, thermometer]) => {
        if (
          thermometer.last_measurement !== null &&
          thermometer.target_temperature !== null
        ) {
          x.push(date);
          temperature.push(thermometer.last_measurement);
          cible.push(thermometer.target_temperature);
        }
      });

      this.data = [x, temperature, cible];
    };

    poll_history();
    poll_status();
    setInterval(poll_history, 3000);
    setInterval(poll_status, 3000);
  },
});
</script>
