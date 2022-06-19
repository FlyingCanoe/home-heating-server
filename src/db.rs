use std::collections::HashMap;

use chrono::Local;
use rusqlite::params;
use tokio::sync::watch;

use crate::Thermometer;

fn create_table(conn: &rusqlite::Connection) {
    let statements = include_str!("create-table.sql");
    conn.execute_batch(statements).unwrap();
}

fn insert_thermometer_history(
    conn: &rusqlite::Connection,
    name: &str,
    time: &chrono::DateTime<Local>,
    thermometer: &Thermometer,
) {
    let time = time.to_rfc3339();
    let status = thermometer.status.to_string();
    let last_measurement = thermometer
        .last_measurement
        .as_ref()
        .map(f64::to_string)
        .unwrap_or("null".to_string());

    let target_temperature = thermometer
        .target_temperature
        .as_ref()
        .map(f64::to_string)
        .unwrap_or("null".to_string());

    conn.execute(
        "INSERT INTO thermometer_history VALUES(?, ?, ?, ?)",
        params![time, status, last_measurement, target_temperature],
    )
    .unwrap();
}

pub(crate) fn start_db(
    watch: watch::Receiver<HashMap<String, Thermometer>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let conn = rusqlite::Connection::open("db.sqlite").unwrap();
        create_table(&conn);

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            for (name, thermometer) in watch.borrow().iter() {
                let time = chrono::Local::now();

                insert_thermometer_history(&conn, name, &time, thermometer)
            }

            interval.tick().await;
        }
    })
}
