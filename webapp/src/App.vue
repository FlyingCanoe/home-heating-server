<template>
  <table>
    <thead>
      <tr>
        <th>Nom</th>
        <th>Status</th>
        <th>Température</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="thermometer in thermometer_list" v-bind:key="thermometer[0]">
        <td v-text="thermometer[0]" />
        <td v-text="thermometer[1].status" />
        <td v-text="thermometer[1].last_measurement" />
      </tr>
    </tbody>
  </table>
</template>

<style>
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

  created() {
    let poll_api = async () => {
      const response = await fetch("/rest-api/thermometer");
      const thermometer_list: [string, thermometer][] = await response.json();
      this.thermometer_list = thermometer_list;
    };
    poll_api();
    setInterval(poll_api, 3000);
  },
});
</script>
