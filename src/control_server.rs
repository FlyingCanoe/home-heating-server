use std::collections::HashMap;

use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use tokio::sync::watch;

use super::{Thermometer, ThermometerStatus};

fn handle_msg(msg: rumqttc::Publish, sender: &mut watch::Sender<HashMap<String, Thermometer>>) {
    let topic_path: Vec<_> = msg.topic.split('/').collect();

    if topic_path[0] == "thermometer" && topic_path.len() == 3 {
        let name = topic_path[1];
        let msg_type = topic_path[2];
        match msg_type {
            "status" => {
                let payload = String::from_utf8(msg.payload.to_vec()).expect("bad message");
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
                let measurement: f64 = String::from_utf8(msg.payload.to_vec())
                    .expect("bad message")
                    .parse()
                    .expect("bad message");

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
                let target_temperature: f64 = String::from_utf8(msg.payload.to_vec())
                    .expect("bad msg")
                    .parse()
                    .expect("bad msg");

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

pub(crate) fn start_control_server(
    mut sender: watch::Sender<HashMap<String, Thermometer>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mqtt_options = MqttOptions::new("server", "127.0.0.1", 1883);
        let (client, mut connection) = AsyncClient::new(mqtt_options, 10);

        client
            .subscribe("thermometer/#".to_string(), QoS::ExactlyOnce)
            .await
            .unwrap();

        loop {
            let notification = connection.poll().await.unwrap();
            if let Event::Incoming(Packet::Publish(msg)) = notification {
                handle_msg(msg, &mut sender)
            }
        }
    })
}
