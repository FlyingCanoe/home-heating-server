use std::{collections::HashMap, time::Duration};

use rumqttc::{Client, MqttOptions, QoS, SubscribeFilter};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum ThermometerStatus {
    Connected,
    Disconnected,
}

fn handle_msg(msg: rumqttc::Publish, thermometer_list: &mut HashMap<String, ThermometerStatus>) {
    let topic_path: Vec<_> = msg.topic.split('/').collect();

    if topic_path[0] == "thermometer" && topic_path.len() == 2 {
        let payload =
            String::from_utf8(msg.payload.to_vec()).expect(&format!("bad message {:?}", msg));

        let status = match payload.as_str() {
            "connected" => ThermometerStatus::Connected,
            "disconnected" => ThermometerStatus::Disconnected,
            _ => panic!("bad message {:?}", msg),
        };

        *thermometer_list
            .entry(topic_path[1].to_string())
            .or_insert(status) = status;
    }

    dbg!(thermometer_list);
}

fn main() {
    let mut mqttoptions = MqttOptions::new("server", "127.0.0.1", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (mut client, mut connection) = Client::new(mqttoptions, 10);
    client
        .subscribe_many([SubscribeFilter::new(
            "thermometer/+".to_string(),
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
