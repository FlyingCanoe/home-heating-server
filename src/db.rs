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
    let last_measurement = thermometer.last_measurement;
    let target_temperature = thermometer.target_temperature;

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
                status: ThermometerStatus::from(row.get::<_, String>(1).unwrap()),
                last_measurement: row.get(2).unwrap(),
                target_temperature: row.get::<_, Option<f64>>(3).unwrap(),
            };
            Ok((time, thermometer))
        })
        .unwrap()
        .map(|x| if let Ok(x) = x { x } else { panic!("test 3") })
        .collect()
}

pub(crate) fn start_db(
    watch: watch::Receiver<HashMap<String, Thermometer>>,
    mut rx: mpsc::UnboundedReceiver<DbRequest>,
    tx: mpsc::UnboundedSender<DbResponse>,
) {
    let conn = rusqlite::Connection::open("db.sqlite").unwrap();
    create_table(&conn);

    std::thread::spawn(move || loop {
        while let Ok(request) = rx.try_recv() {
            match request {
                DbRequest::ThermometerHistory(name) => {
                    let response = get_thermometer_history(&conn, name.as_str());
                    tx.send(DbResponse::ThermometerHistory(response)).unwrap();
                }
            }
        }

        for (name, thermometer) in watch.borrow().iter() {
            let time = chrono::Local::now();
            insert_thermometer_history(&conn, name, &time, thermometer)
        }
        std::thread::sleep_ms(10 * 1000);
    });
}
