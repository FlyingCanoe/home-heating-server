use rumqttc::{Client, MqttOptions, QoS, SubscribeFilter};
use std::time::Duration;

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

    for notification in connection.iter() {
        match notification.unwrap() {
            rumqttc::Event::Incoming(packet) => match packet {
                rumqttc::Packet::Publish(msg) => {
                    println!(
                        "topic={}, payload={}",
                        msg.topic,
                        String::from_utf8(msg.payload.to_vec()).unwrap()
                    );
                }
                _ => {}
            },
            _ => {}
        }
    }
}
