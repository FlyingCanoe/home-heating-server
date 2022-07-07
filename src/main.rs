use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;
use tokio::sync::watch;

mod control_server;
mod db;
mod web_server;

use control_server::start_control_server;
use db::start_db;
use web_server::start_web_server;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thermometer {
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

impl From<String> for ThermometerStatus {
    fn from(input: String) -> Self {
        match input.as_str() {
            "Connected" => ThermometerStatus::Connected,
            "Disconnected" => ThermometerStatus::Disconnected,
            _ => panic!("invalid status"),
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let (watch, watcher) = watch::channel(HashMap::<String, Thermometer>::new());

    let (control_tx, control_rx) = tokio::sync::mpsc::unbounded_channel();
    let (db_request_tx, db_request_rx) = tokio::sync::mpsc::unbounded_channel();
    let (db_response_tx, db_response_rx) = tokio::sync::mpsc::unbounded_channel();

    start_web_server(watcher.clone(), control_tx, db_request_tx, db_response_rx);
    start_db(watcher, db_request_rx, db_response_tx);
    start_control_server(watch, control_rx).await.unwrap();
}
