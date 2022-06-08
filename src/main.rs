use std::collections::HashMap;

use tokio::try_join;
use warp::Filter;

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
                let payload = String::from_utf8(msg.payload.to_vec()).expect("bad message");
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

async fn start_web_server() {
    let filter = warp::path::end().map(|| "Hello, world!".to_string());
    warp::serve(filter).run(([127, 0, 0, 1], 3030)).await;
}

async fn start_control_server() {
    use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};

    let mqtt_options = MqttOptions::new("server", "127.0.0.1", 1883);
    let (client, mut connection) = AsyncClient::new(mqtt_options, 10);

    client
        .subscribe("thermometer/#".to_string(), QoS::ExactlyOnce)
        .await
        .unwrap();

    let mut thermometer_list = HashMap::new();
    while let Ok(notification) = connection.poll().await {
        if let Event::Incoming(Packet::Publish(msg)) = notification {
            handle_msg(msg, &mut thermometer_list)
        }
    }
}

#[tokio::main]
async fn main() {
    let web_server = tokio::spawn(start_web_server());
    let control_server = tokio::spawn(start_control_server());

    try_join!(web_server, control_server).unwrap();
}
