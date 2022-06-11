export interface thermometer {
  status: "connected" | "disconnected";
  last_measurement: number | null;
}
