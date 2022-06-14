use std::collections::HashMap;

use tokio::sync::mpsc;
use tokio::sync::watch;
use warp::Filter;
use warp::Rejection;

use super::Thermometer;

fn thermometer_status(
    receiver: watch::Receiver<HashMap<String, Thermometer>>,
) -> impl Filter<Error = Rejection, Extract = (warp::reply::Json,)> + Clone {
    warp::get()
        .and(warp::path!("rest-api" / "thermometer-status"))
        .map(move || {
            let thermometer_list: Vec<_> = (receiver).borrow().clone().into_iter().collect();
            warp::reply::json(&thermometer_list)
        })
}

fn change_thermometer_target(
    sender: mpsc::UnboundedSender<(String, f64)>,
) -> impl Filter<Error = Rejection, Extract = (String,)> + Clone {
    warp::post()
        .and(warp::path!("rest-api" / "thermometer-target"))
        .and(warp::body::json())
        .map(move |body: (String, f64)| {
            sender.send(body).unwrap();
            "".to_string()
        })
}

pub(crate) fn start_web_server(
    receiver: watch::Receiver<HashMap<String, Thermometer>>,
    sender: mpsc::UnboundedSender<(String, f64)>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async {
        let rest_api = thermometer_status(receiver).or(change_thermometer_target(sender));

        let webapp = warp::get().and(warp::fs::dir("webapp/dist"));

        let filter = rest_api.or(webapp);
        warp::serve(filter).run(([127, 0, 0, 1], 5500)).await
    })
}
