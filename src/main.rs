use std::time::Duration;

use control_server::ControlState;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::watch;

mod control_server;
mod db;
mod web_server;

use control_server::start_control_server;
use db::start_db;
use web_server::start_web_server;

const SLEEP_TIME: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thermometer {
    status: ThermometerStatus,
    /// last measurement of the thermometer in degree Celsius.
    last_measurement: Option<f64>,
    target_temperature: Option<f64>,
}

impl Thermometer {
    pub fn is_disconnected(&self) -> bool {
        match self.status {
            ThermometerStatus::Connected => false,
            ThermometerStatus::Disconnected => true,
        }
    }

    pub fn is_below_target(&self) -> bool {
        let Some(last_measurement) = self.last_measurement else {return false};
        let Some(target_temperature) = self.target_temperature else {return false};

        last_measurement < target_temperature
    }
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

    let (watch, watcher) = watch::channel(ControlState::default());

    let (control_tx, control_rx) = tokio::sync::mpsc::unbounded_channel();
    let (db_request_tx, db_request_rx) = tokio::sync::mpsc::unbounded_channel();
    let (db_response_tx, db_response_rx) = tokio::sync::mpsc::unbounded_channel();

    start_web_server(watcher.clone(), control_tx, db_request_tx, db_response_rx);
    start_db(watcher, db_request_rx, db_response_tx);
    start_control_server(watch, control_rx).await;

    loop {
        tokio::time::sleep(SLEEP_TIME).await
    }
}
