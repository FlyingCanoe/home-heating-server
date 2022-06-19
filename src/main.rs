use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;
use tokio::select;
use tokio::sync::watch;

mod control_server;
mod db;
mod web_server;

use control_server::start_control_server;
use db::start_db;
use web_server::start_web_server;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Thermometer {
    status: ThermometerStatus,
    /// last measurement of the thermometer in degree Celsius.
    last_measurement: Option<f64>,
    target_temperature: Option<f64>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
enum ThermometerStatus {
    Connected,
    Disconnected,
}

impl ToString for ThermometerStatus {
    fn to_string(&self) -> String {
        match self {
            ThermometerStatus::Connected => "Connected",
            ThermometerStatus::Disconnected => "Disconnected",
        }
        .to_string()
    }
}

#[tokio::main]
async fn main() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let (sender, receiver) = watch::channel(HashMap::<String, Thermometer>::new());

    let web_server = start_web_server(receiver.clone(), tx);
    let control_server = start_control_server(sender, rx);
    let db = start_db(receiver);

    select! {
        output = web_server => {output}
        output = control_server => {output}
        output = db => {output}
    }
    .unwrap()
}
