use std::collections::HashMap;
use std::time::Duration;

use actix_web::web::Bytes;
use rumqttc::LastWill;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::sync::watch;

use crate::SLEEP_TIME;

use super::{Thermometer, ThermometerStatus};

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

fn get_error(
    status: &HashMap<String, Thermometer>,
    heater_connected: &watch::Receiver<bool>,
) -> Option<Error> {
    if !*heater_connected.borrow() {
        let error = Error {
            msg: "le chauffage est déconnecté".to_string(),
            severity: ErrorSeverity::Error,
        };
        Some(error)
    } else if status
        .iter()
        .all(|(_, thermometer)| thermometer.is_disconnected())
    {
        let error = Error {
            msg: "tous les thermomètre sont déconnecté".to_string(),
            severity: ErrorSeverity::Error,
        };
        Some(error)
    } else if status
        .iter()
        .any(|(_, thermometer)| thermometer.is_disconnected())
    {
        let error = Error {
            msg: "certain thermomètre sont déconnecté".to_string(),
            severity: ErrorSeverity::Warning,
        };
        Some(error)
    } else {
        None
    }
}

fn handle_msg(
    msg: rumqttc::Publish,
    sender: &mut watch::Sender<HashMap<String, Thermometer>>,
    heater_connected_mut: &mut watch::Sender<bool>,
) {
    let topic_path: Vec<_> = msg.topic.split('/').collect();
    let payload = String::from_utf8(msg.payload.to_vec()).expect("bad message");

    if msg.topic == "heater/status" {
        match payload.as_str() {
            "connected" => heater_connected_mut.send(true).unwrap(),
            "disconnected" => heater_connected_mut.send(false).unwrap(),
            _ => panic!("bad msg"),
        }
    }

    if topic_path[0] == "thermometer" && topic_path.len() == 3 {
        let name = topic_path[1];
        let msg_type = topic_path[2];
        match msg_type {
            "status" => {
                let status = match payload.as_str() {
                    "connected" => ThermometerStatus::Connected,
                    "disconnected" => ThermometerStatus::Disconnected,
                    _ => panic!("bad message {:?}", msg),
                };

                sender.send_modify(|thermometer_list| {
                    thermometer_list
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
                let measurement: f64 = payload.parse().expect("bad message");

                sender.send_modify(|thermometer_list| {
                    thermometer_list
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
                let target_temperature: f64 = payload.parse().expect("bad msg");

                sender.send_modify(|thermometer_list| {
                    thermometer_list
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

async fn publish_error(
    client: &AsyncClient,
    thermometer: &watch::Receiver<HashMap<String, Thermometer>>,
    error_mut: &mut watch::Sender<Option<Error>>,
    heater_connected: &watch::Receiver<bool>,
) {
    let error = get_error(&thermometer.borrow(), heater_connected);

    if let Some(ref error) = error {
        match &error.severity {
            ErrorSeverity::Warning => client
                .publish("error", QoS::AtLeastOnce, true, "warning")
                .await
                .unwrap(),
            ErrorSeverity::Error => client
                .publish("error", QoS::AtMostOnce, false, "error")
                .await
                .unwrap(),
        }
    } else {
        client
            .publish("error", QoS::AtLeastOnce, true, "")
            .await
            .unwrap();
    }

    error_mut.send(error).unwrap();
}

async fn handle_webserver_request(
    receiver: &mut mpsc::UnboundedReceiver<(String, f64)>,
    client: &AsyncClient,
) {
    if let Ok((name, wanted_temperature)) = receiver.try_recv() {
        client
            .publish(
                format!("thermometer/{name}/change-target"),
                QoS::ExactlyOnce,
                false,
                wanted_temperature.to_string(),
            )
            .await
            .unwrap();
    };
}

pub(crate) async fn start_control_server(
    mut thermometer_sender: watch::Sender<HashMap<String, Thermometer>>,
    mut error_sender: watch::Sender<Option<Error>>,
    mut receiver: mpsc::UnboundedReceiver<(String, f64)>,
) {
    let mut mqtt_options = MqttOptions::new("server", "127.0.0.1", 1883);
    mqtt_options.set_last_will(LastWill {
        topic: "error".to_string(),
        message: Bytes::from_static(b"error"),
        qos: QoS::AtLeastOnce,
        retain: true,
    });
    let (client, mut connection) = AsyncClient::new(mqtt_options, 10);

    client
        .subscribe("thermometer/#".to_string(), QoS::AtLeastOnce)
        .await
        .unwrap();

    client
        .subscribe("heater/status", QoS::AtLeastOnce)
        .await
        .unwrap();

    let thermometer_watcher = thermometer_sender.subscribe();
    let (mut heater_connected_mut, heater_connected) = watch::channel(false);
    tokio::spawn(async move {
        loop {
            if let Event::Incoming(Packet::Publish(msg)) = connection.poll().await.unwrap() {
                handle_msg(msg, &mut thermometer_sender, &mut heater_connected_mut)
            }
        }
    });
    tokio::task::spawn(async move {
        let mut last_error_publish = std::time::Instant::now();
        let s = thermometer_watcher;
        loop {
            if last_error_publish.elapsed() >= std::time::Duration::from_secs(2) {
                publish_error(&client, &s, &mut error_sender, &heater_connected).await;

                last_error_publish = std::time::Instant::now();
                update_heater_control(&client, &s).await;
            }

            handle_webserver_request(&mut receiver, &client).await;

            tokio::time::sleep(SLEEP_TIME).await;
        }
    });
}

async fn update_heater_control(
    client: &AsyncClient,
    thermometers: &watch::Receiver<HashMap<String, Thermometer>>,
) {
    let heating_neaded = thermometers.borrow().iter().any(|(_, thermometer)| {
        if let (Some(last_measurement), Some(target)) =
            (thermometer.last_measurement, thermometer.target_temperature)
        {
            last_measurement < target
        } else {
            false
        }
    });

    if heating_neaded {
        client
            .publish("heating", QoS::AtLeastOnce, false, "on")
            .await
            .unwrap();
    } else {
        client
            .publish("heating", QoS::AtLeastOnce, false, "off")
            .await
            .unwrap();
    }
}
