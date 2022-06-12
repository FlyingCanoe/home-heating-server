use std::collections::HashMap;
use std::future::Future;

use tokio::sync::watch;
use warp::Filter;

use super::Thermometer;

pub(crate) async fn start_web_server(
    receiver: watch::Receiver<HashMap<String, Thermometer>>,
) -> impl Future<Output = ()> {
    let rest_api = warp::path!("rest-api" / "thermometer").map(move || {
        let thermometer_list: Vec<_> = (&receiver).borrow().clone().into_iter().collect();
        warp::reply::json(&thermometer_list)
    });

    let webapp = warp::fs::dir("webapp/dist");

    let filter = warp::get().and(rest_api.or(webapp));
    warp::serve(filter).run(([127, 0, 0, 1], 3030))
}
