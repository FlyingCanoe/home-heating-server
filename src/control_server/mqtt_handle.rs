use actix_web::web::Bytes;
use rumqttc::{AsyncClient, Event, EventLoop, LastWill, MqttOptions, Packet, QoS};
use tokio::sync::mpsc::{self, error::TryRecvError};

use super::{Error, ErrorSeverity};

#[derive(Debug, Clone)]
pub struct MqttMsg {
    pub topic: String,
    pub path: Vec<String>,
    pub payload: String,
}

#[derive(Debug)]
pub struct MqttHandle {
    client: AsyncClient,
    msg_queue: mpsc::UnboundedReceiver<MqttMsg>,
}

impl MqttHandle {
    pub async fn new() -> Self {
        let mut mqtt_options = MqttOptions::new("server", "127.0.0.1", 1883);
        mqtt_options.set_last_will(LastWill {
            topic: "error".to_string(),
            message: Bytes::from_static(b"error"),
            qos: QoS::AtLeastOnce,
            retain: true,
        });
        let (client, event_loop) = AsyncClient::new(mqtt_options, 10);

        client
            .subscribe("thermometer/#".to_string(), QoS::AtLeastOnce)
            .await
            .unwrap();

        client
            .subscribe("heater/status", QoS::AtLeastOnce)
            .await
            .unwrap();

        let msg_queue = start_event_loop(event_loop).await;

        MqttHandle { client, msg_queue }
    }

    pub fn read_mqtt_msg_list(&mut self) -> Vec<MqttMsg> {
        let mut output = vec![];

        loop {
            match self.msg_queue.try_recv() {
                Ok(msg) => output.push(msg),
                Err(TryRecvError::Empty) => break,
                Err(_) => panic!(),
            }
        }

        output
    }

    pub async fn send_change_temperature_msg(&self, thermometer_name: String, temperature: f64) {
        self.client
            .publish(
                format!("thermometer/{thermometer_name}/change-target"),
                QoS::ExactlyOnce,
                false,
                temperature.to_string(),
            )
            .await
            .unwrap();
    }

    pub async fn publish_heating_status(&self, on: bool) {
        let payload;
        if on {
            payload = "on";
        } else {
            payload = "off";
        }

        self.client
            .publish("heating", QoS::AtLeastOnce, false, payload)
            .await
            .unwrap();
    }

    pub async fn publish_error(&self, error: Option<Error>) {
        let payload;
        if let Some(ref error) = error {
            match &error.severity {
                ErrorSeverity::Warning => payload = "warning",
                ErrorSeverity::Error => payload = "error",
            }
        } else {
            payload = "";
        }

        self.client
            .publish("error", QoS::AtLeastOnce, true, payload)
            .await
            .unwrap();
    }
}

async fn start_event_loop(mut event_loop: EventLoop) -> mpsc::UnboundedReceiver<MqttMsg> {
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        loop {
            if let Event::Incoming(Packet::Publish(msg)) = event_loop.poll().await.unwrap() {
                let msg = MqttMsg {
                    path: msg.topic.split("/").map(String::from).collect(),
                    topic: msg.topic,
                    payload: String::from_utf8(msg.payload.to_vec()).expect("bad msg"),
                };
                tx.send(msg).unwrap();
            }
        }
    });

    rx
}
