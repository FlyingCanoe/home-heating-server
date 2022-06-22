use std::collections::HashMap;

use chrono::DateTime;
use chrono::FixedOffset;
use chrono::Local;
use rusqlite::params;
use tokio::sync::{mpsc, watch};

use crate::Thermometer;
use crate::ThermometerStatus;

#[derive(Clone, Debug)]

pub enum DbRequest {
    ThermometerHistory(String),
}

#[derive(Clone, Debug)]
pub enum DbResponse {
    ThermometerHistory(Vec<(DateTime<FixedOffset>, Thermometer)>),
}

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
        "INSERT INTO thermometer_history VALUES(?, ?, ?, ?, ?)",
        params![time, name, status, last_measurement, target_temperature],
    )
    .unwrap();
}

fn get_thermometer_history(
    conn: &rusqlite::Connection,
    name: &str,
) -> Vec<(DateTime<FixedOffset>, Thermometer)> {
    let mut statement = conn
        .prepare(
            "SELECT
                time,
                status,
                last_measurement,
                target_temperature
            FROM thermometer_history
            WHERE name = ?",
        )
        .unwrap();
    statement
        .query_map(params![name], |row| {
            // get the column 0 (time) and parse it as a DateTime
            let time =
                DateTime::parse_from_rfc3339(row.get::<_, String>(0).unwrap().as_str()).unwrap();
            let thermometer = Thermometer {
                // get column 1 (status) and parse it as a ThermometerStatus
                status: ThermometerStatus::from(row.get::<_, String>(1)?),
                last_measurement: row.get(2)?,
                target_temperature: row.get(3)?,
            };
            Ok((time, thermometer))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

pub(crate) fn start_db(
    watch: watch::Receiver<HashMap<String, Thermometer>>,
    mut rx: mpsc::UnboundedReceiver<DbRequest>,
    tx: mpsc::UnboundedSender<DbResponse>,
) {
    std::thread::spawn(move || {
        let conn = rusqlite::Connection::open("db.sqlite").unwrap();
        create_table(&conn);

        //let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));

        while let Some(request) = rx.blocking_recv() {
            match request {
                DbRequest::ThermometerHistory(name) => {
                    let response = get_thermometer_history(&conn, name.as_str());
                    tx.send(DbResponse::ThermometerHistory(response)).unwrap();
                }
            }
        }

        loop {
            for (name, thermometer) in watch.borrow().iter() {
                let time = chrono::Local::now();

                insert_thermometer_history(&conn, name, &time, thermometer)
            }

            std::thread::sleep_ms(10 * 1000);
        }
    });
}
