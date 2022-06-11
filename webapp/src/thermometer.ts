export interface thermometer {
  status: "connected" | "disconnected";
  last_measurement: number | null;
  target_temperature: number | null;
}
