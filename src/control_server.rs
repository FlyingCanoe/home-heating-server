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
}

fn handle_msg(msg: MqttMsg, sender: &mut watch::Sender<ControlState>) {
    if msg.topic == "heater/status" {
        match msg.payload.as_str() {
            "connected" => sender.send_modify(|state| state.heater_connected = true),
            "disconnected" => sender.send_modify(|state| state.heater_connected = false),
            _ => panic!("bad msg"),
        }
    }

    if msg.path[0] == "thermometer" && msg.path.len() == 3 {
        let name = msg.path[1].as_str();
        let msg_type = msg.path[2].as_str();
        match msg_type {
            "status" => {
                let status = match msg.payload.as_str() {
                    "connected" => ThermometerStatus::Connected,
                    "disconnected" => ThermometerStatus::Disconnected,
                    _ => panic!("bad message {:?}", msg),
                };

                sender.send_modify(|state| {
                    state
                        .thermometer_list
                        .entry(name.to_string())
                        .or_insert(Thermometer {
                            status,
                            last_measurement: None,
                            target_temperature: None,
                        })
                        .status = status;
                });
            }
            "measurement" => {
                let measurement: f64 = msg.payload.parse().expect("bad message");

                sender.send_modify(|state| {
                    state
                        .thermometer_list
                        .entry(name.to_string())
                        .or_insert(Thermometer {
                            status: ThermometerStatus::Disconnected,
                            last_measurement: Some(measurement),
                            target_temperature: None,
                        })
                        .last_measurement = Some(measurement);
                });
            }
            "target-temperature" => {
                let target_temperature: f64 = msg.payload.parse().expect("bad msg");

                sender.send_modify(|state| {
                    state
                        .thermometer_list
                        .entry(name.to_string())
                        .or_insert(Thermometer {
                            status: ThermometerStatus::Disconnected,
                            last_measurement: Some(target_temperature),
                            target_temperature: None,
                        })
                        .target_temperature = Some(target_temperature);
                });
            }
            _ => {}
        }
    }
}

async fn publish_error(mqtt_handle: &MqttHandle, state: &mut watch::Sender<ControlState>) {
    state.send_modify(|state| state.update_error());

    let error = state.borrow().error.clone();
    mqtt_handle.publish_error(error).await
}

pub(crate) async fn start_control_server(
    mut state: watch::Sender<ControlState>,
    mut receiver: mpsc::UnboundedReceiver<(String, f64)>,
) {
    let thermometer_watcher = state.subscribe();

    let mut mqtt_handle = MqttHandle::new().await;
    tokio::task::spawn(async move {
        let mut last_error_publish = std::time::Instant::now();
        let s = thermometer_watcher;
        loop {
            let msg_list = mqtt_handle.read_mqtt_msg_list();
            let request_list = read_request_list(&mut receiver);

            handle_request_list(&mut mqtt_handle, request_list).await;
            handle_mqtt_msg_list(msg_list, &mut state).await;

            if last_error_publish.elapsed() >= std::time::Duration::from_secs(2) {
                publish_error(&mqtt_handle, &mut state).await;

                last_error_publish = std::time::Instant::now();
                update_heater_control(&mqtt_handle, &s).await;
            }

            tokio::time::sleep(SLEEP_TIME).await;
        }
    });
}

async fn handle_request_list(mqtt_handle: &mut MqttHandle, request_list: Vec<(String, f64)>) {
    for (name, target_temperature) in request_list {
        mqtt_handle
            .send_change_temperature_msg(name, target_temperature)
            .await
    }
}

async fn handle_mqtt_msg_list(msg_list: Vec<MqttMsg>, sender: &mut watch::Sender<ControlState>) {
    for msg in msg_list {
        handle_msg(msg, sender)
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

async fn update_heater_control(mqtt_handle: &MqttHandle, state: &watch::Receiver<ControlState>) {
    let heating_needed = state
        .borrow()
        .thermometer_list
        .iter()
        .any(|(_, thermometer)| {
            if let (Some(last_measurement), Some(target)) =
                (thermometer.last_measurement, thermometer.target_temperature)
            {
                last_measurement < target
            } else {
                false
            }
        });

    mqtt_handle.publish_heating_status(heating_needed).await;
}
