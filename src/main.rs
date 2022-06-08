use std::collections::HashMap;

use tokio::sync::watch;
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
                        })
                        .status = status;
                });
            }
            "measurement" => {
                let measurement: f64 = String::from_utf8(msg.payload.to_vec())
                    .unwrap()
                    .parse()
                    .unwrap();

                sender.send_modify(|thermometer_list| {
                    thermometer_list
                        .entry(name.to_string())
                        .or_insert(Thermometer {
                            status: ThermometerStatus::Disconnected,
                            last_measurement: Some(measurement),
                        })
                        .last_measurement = Some(measurement);
                });
            }
            _ => {}
        }
    }
}

async fn start_web_server(receiver: watch::Receiver<HashMap<String, Thermometer>>) {
    let filter = warp::path::end().map(move || {
        let thermometer_list = (&receiver).borrow();
        let page: String = thermometer_list
            .iter()
            .map(|(name, thermometer)| {
                format!(
                    "name={name}, status={:?}, last_measurement={:?}\n",
                    thermometer.status, thermometer.last_measurement
                )
            })
            .collect();
        page
    });
    warp::serve(filter).run(([127, 0, 0, 1], 3030)).await;
}

async fn start_control_server(mut sender: watch::Sender<HashMap<String, Thermometer>>) {
    use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};

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
}

#[tokio::main]
async fn main() {
    let (sender, receiver) = watch::channel(HashMap::<String, Thermometer>::new());

    let web_server = tokio::spawn(start_web_server(receiver));
    let control_server = tokio::spawn(start_control_server(sender));

    try_join!(web_server, control_server).unwrap();
}
