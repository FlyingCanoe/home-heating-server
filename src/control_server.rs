use std::collections::HashMap;

use serde::Serialize;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::watch;

use crate::SLEEP_TIME;

use self::mqtt_handle::MqttHandle;

use super::{Thermometer, ThermometerStatus};
use mqtt_handle::MqttMsg;

mod mqtt_handle;

#[derive(Debug, Clone, Serialize)]
pub struct Error {
    msg: String,
    severity: ErrorSeverity,
}

#[derive(Debug, Clone, Serialize)]
pub enum ErrorSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ControlState {
    pub thermometer_list: HashMap<String, Thermometer>,
    pub error: Option<Error>,
    heater_connected: bool,
    heating_needed: bool,
}

impl ControlState {
    fn update_error(&mut self) {
        let new_error;
        if !self.heater_connected {
            let error = Error {
                msg: "le chauffage est déconnecté".to_string(),
                severity: ErrorSeverity::Error,
            };
            new_error = Some(error)
        } else if self
            .thermometer_list
            .iter()
            .all(|(_, thermometer)| thermometer.is_disconnected())
        {
            let error = Error {
                msg: "tous les thermomètre sont déconnecté".to_string(),
                severity: ErrorSeverity::Error,
            };
            new_error = Some(error)
        } else if self
            .thermometer_list
            .iter()
            .any(|(_, thermometer)| thermometer.is_disconnected())
        {
            let error = Error {
                msg: "certain thermomètre sont déconnecté".to_string(),
                severity: ErrorSeverity::Warning,
            };
            new_error = Some(error)
        } else {
            new_error = None
        }

        self.error = new_error;
    }

    fn update_heating_needed(&mut self) {
        self.heating_needed = self
            .thermometer_list
            .values()
            .any(Thermometer::is_below_target)
    }

    fn handle_msg(&mut self, msg: MqttMsg) {
        if msg.topic == "heater/status" {
            match msg.payload.as_str() {
                "connected" => self.heater_connected = true,
                "disconnected" => self.heater_connected = false,
                _ => panic!("bad msg"),
            }
        }

        if msg.path[0] == "thermometer" && msg.path.len() == 3 {
            let name = msg.path[1].as_str();
            let msg_type = msg.path[2].as_str();

            let thermometer =
                self.thermometer_list
                    .entry(name.to_string())
                    .or_insert(Thermometer {
                        status: ThermometerStatus::Disconnected,
                        last_measurement: None,
                        target_temperature: None,
                    });

            match msg_type {
                "status" => {
                    let status = match msg.payload.as_str() {
                        "connected" => ThermometerStatus::Connected,
                        "disconnected" => ThermometerStatus::Disconnected,
                        _ => panic!("bad message {:?}", msg),
                    };

                    thermometer.status = status;
                }
                "measurement" => {
                    let measurement: f64 = msg.payload.parse().expect("bad message");

                    thermometer.last_measurement = Some(measurement);
                }
                "target-temperature" => {
                    let target_temperature: f64 = msg.payload.parse().expect("bad msg");

                    thermometer.target_temperature = Some(target_temperature);
                }
                _ => {}
            }
        }
    }

    fn update(&mut self, msg_list: Vec<MqttMsg>) {
        for msg in msg_list {
            self.handle_msg(msg)
        }

        self.update_error();
        self.update_heating_needed()
    }
}

pub(crate) async fn start_control_server(
    state_mut: watch::Sender<ControlState>,
    mut receiver: mpsc::UnboundedReceiver<(String, f64)>,
) {
    let mut mqtt_handle = MqttHandle::new().await;
    let state_ref = state_mut.subscribe();

    tokio::task::spawn(async move {
        loop {
            let msg_list = mqtt_handle.read_mqtt_msg_list();
            let request_list = read_request_list(&mut receiver);

            state_mut.send_modify(|state| state.update(msg_list));

            handle_request_list(&mut mqtt_handle, request_list).await;

            mqtt_handle.publish_state(&state_ref).await;

            tokio::time::sleep(SLEEP_TIME).await;
        }
    });
}

async fn handle_request_list(mqtt_handle: &mut MqttHandle, request_list: Vec<(String, f64)>) {
    for (name, target_temperature) in request_list {
        mqtt_handle.change_target(name, target_temperature).await
    }
}

fn read_request_list(
    request_queue: &mut mpsc::UnboundedReceiver<(String, f64)>,
) -> Vec<(String, f64)> {
    let mut output = vec![];

    loop {
        match request_queue.try_recv() {
            Ok(msg) => output.push(msg),
            Err(TryRecvError::Empty) => break,
            Err(_) => panic!(),
        }
    }

    output
}
