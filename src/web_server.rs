use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use tokio::sync::mpsc;
use tokio::sync::watch;
use warp::body;
use warp::filters::path;
use warp::filters::BoxedFilter;
use warp::reply;
use warp::Filter;
use warp::Rejection;

use crate::db::{DbRequest, DbResponse};
use crate::Thermometer;

fn thermometer_status(
    receiver: watch::Receiver<HashMap<String, Thermometer>>,
) -> BoxedFilter<(reply::Json,)> {
    warp::get()
        .and(warp::path!("rest-api" / "thermometer-status"))
        .map(move || {
            let thermometer_list: Vec<_> = (receiver).borrow().clone().into_iter().collect();
            reply::json(&thermometer_list)
        })
        .boxed()
}

fn change_thermometer_target(
    sender: mpsc::UnboundedSender<(String, f64)>,
) -> BoxedFilter<(String,)> {
    warp::post()
        .and(warp::path!("rest-api" / "thermometer-target"))
        .and(body::json())
        .map(move |body: (String, f64)| {
            sender.send(body).unwrap();
            "".to_string()
        })
        .boxed()
}

fn thermometer_history(
    tx: mpsc::UnboundedSender<DbRequest>,
    rx: Arc<Mutex<mpsc::UnboundedReceiver<DbResponse>>>,
) -> impl Filter<Error = Rejection, Extract = (String,)> + Clone {
    warp::get()
        .and(warp::path("rest-api"))
        .and(warp::path("thermometer-history"))
        .and(path::param())
        .and(path::end())
        .map(move |name: String| {
            tx.send(DbRequest::ThermometerHistory(name)).unwrap();

            let mut rx = if let Ok(rx) = rx.lock() {
                rx
            } else {
                return "".to_string();
            };
            let t = rx.blocking_recv().unwrap();

            let DbResponse::ThermometerHistory(history) = t;
            serde_json::to_string(&history).unwrap()
        })
}

pub(crate) fn start_web_server(
    watcher: watch::Receiver<HashMap<String, Thermometer>>,
    control_tx: mpsc::UnboundedSender<(String, f64)>,
    db_tx: mpsc::UnboundedSender<DbRequest>,
    db_rx: mpsc::UnboundedReceiver<DbResponse>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        let db_rx = Mutex::new(db_rx);
        let db_rx = Arc::new(db_rx);

        let rest_api = thermometer_status(watcher)
            .or(change_thermometer_target(control_tx))
            .or(thermometer_history(db_tx, db_rx));

        let webapp = warp::get().and(warp::fs::dir("webapp/dist"));

        let filter = rest_api.or(webapp);
        warp::serve(filter).run(([127, 0, 0, 1], 5500)).await
    })
}
