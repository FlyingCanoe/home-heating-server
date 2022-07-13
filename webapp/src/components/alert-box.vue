<template>
  <div v-bind:class="getClass" v-if="alert">
    <h4>
      {{ alert.severity == "Error" ? "erreur" : "avertissement" }}
    </h4>
    <p>
      {{ alert.msg }}
    </p>
  </div>
</template>

<style lang="scss">
@import "../assets/base.scss";
</style>

<script lang="ts">
import { defineComponent } from "vue";

interface Alert {
  msg: string;
  severity: "Warning" | "Error";
}

export default defineComponent({
  data() {
    return {
      alert: null as Alert | null,
    };
  },
  computed: {
    getClass() {
      if (this.alert != null) {
        if (this.alert.severity == "Error") {
          return "alert-error";
        } else {
          return "alert-warning";
        }
      } else {
        return "";
      }
    },
  },

  created() {
    let poll_alert = async () => {
      const response = await fetch("/rest-api/get-error");
      this.alert = await response.json();
    };

    poll_alert();
    setInterval(poll_alert, 3000);
  },
});
</script>
