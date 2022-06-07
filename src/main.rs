use std::{collections::HashMap, time::Duration};

use rumqttc::{Client, MqttOptions, QoS, SubscribeFilter};

#[derive(Debug, Clone)]
struct Thermometer {
    status: ThermometerStatus,
    /// last measurement of the thermometer in degree Celsius.
    last_measurement: Option<f64>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum ThermometerStatus {
    Connected,
    Disconnected,
}

fn handle_msg(msg: rumqttc::Publish, thermometer_list: &mut HashMap<String, Thermometer>) {
    let topic_path: Vec<_> = msg.topic.split('/').collect();

    if topic_path[0] == "thermometer" && topic_path.len() == 3 {
        let name = topic_path[1];
        let msg_type = topic_path[2];
        match msg_type {
            "status" => {
                let payload = String::from_utf8(msg.payload.to_vec())
                    .expect(&format!("bad message {:?}", msg));

                let status = match payload.as_str() {
                    "connected" => ThermometerStatus::Connected,
                    "disconnected" => ThermometerStatus::Disconnected,
                    _ => panic!("bad message {:?}", msg),
                };

                thermometer_list
                    .entry(name.to_string())
                    .or_insert(Thermometer {
                        status,
                        last_measurement: None,
                    })
                    .status = status;
            }
            "measurement" => {
                let measurement: f64 = String::from_utf8(msg.payload.to_vec())
                    .unwrap()
                    .parse()
                    .unwrap();

                thermometer_list
                    .entry(name.to_string())
                    .or_insert(Thermometer {
                        status: ThermometerStatus::Disconnected,
                        last_measurement: Some(measurement),
                    })
                    .last_measurement = Some(measurement);
            }
            _ => {}
        }
        dbg!(thermometer_list);
    }
}

fn main() {
    let mut mqttoptions = MqttOptions::new("server", "127.0.0.1", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (mut client, mut connection) = Client::new(mqttoptions, 10);
    client
        .subscribe_many([SubscribeFilter::new(
            "thermometer/#".to_string(),
            QoS::ExactlyOnce,
        )])
        .unwrap();

    let mut thermometer_list = HashMap::new();
    for notification in connection.iter() {
        if let rumqttc::Event::Incoming(packet) = notification.unwrap() {
            if let rumqttc::Packet::Publish(msg) = packet {
                handle_msg(msg, &mut thermometer_list)
            }
        }
    }
}
