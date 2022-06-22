use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use actix_files::Files;
use actix_web::{http, web, App, HttpResponse, HttpServer};
use tokio::sync::mpsc;
use tokio::sync::watch;

use crate::db::{DbRequest, DbResponse};
use crate::Thermometer;

#[actix_web::get("/rest-api/thermometer-status")]
async fn thermometer_status(
    receiver: web::Data<watch::Receiver<HashMap<String, Thermometer>>>,
) -> Result<HttpResponse, http::Error> {
    let thermometer_list: Vec<_> = (receiver).borrow().clone().into_iter().collect();
    Ok(HttpResponse::Ok().json(thermometer_list))
}

#[actix_web::post("/rest-api/thermometer-target")]
async fn change_thermometer_target(
    sender: web::Data<mpsc::UnboundedSender<(String, f64)>>,
    body: web::Json<(String, f64)>,
) -> Result<HttpResponse, http::Error> {
    sender.send(body.into_inner()).unwrap();
    Ok(HttpResponse::new(http::StatusCode::OK))
}

#[actix_web::get("/rest-api/thermometer-history/{name}")]
async fn thermometer_history(
    db_conn: web::Data<
        Arc<
            Mutex<(
                mpsc::UnboundedSender<DbRequest>,
                mpsc::UnboundedReceiver<DbResponse>,
            )>,
        >,
    >,
    name: web::Path<String>,
) -> Result<HttpResponse, http::Error> {
    let (tx, rx) = &mut *db_conn.lock().unwrap();

    tx.send(DbRequest::ThermometerHistory(name.into_inner()))
        .unwrap();
    let response = rx.recv().await.unwrap();

    let DbResponse::ThermometerHistory(history) = response;
    serde_json::to_string(&history).unwrap();
    Ok(HttpResponse::Ok().json(history))
}

pub(crate) fn start_web_server(
    watcher: watch::Receiver<HashMap<String, Thermometer>>,
    control_tx: mpsc::UnboundedSender<(String, f64)>,
    db_tx: mpsc::UnboundedSender<DbRequest>,
    db_rx: mpsc::UnboundedReceiver<DbResponse>,
) {
    std::thread::spawn(|| {
        let db_conn = Arc::new(Mutex::new((db_tx, db_rx)));

        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();

        runtime
            .block_on(
                HttpServer::new(move || {
                    App::new()
                        .app_data(web::Data::new(db_conn.clone()))
                        .app_data(web::Data::new(watcher.clone()))
                        .app_data(web::Data::new(control_tx.clone()))
                        .service(thermometer_history)
                        .service(change_thermometer_target)
                        .service(thermometer_status)
                        .service(Files::new("", "webapp/dist").index_file("index.html"))
                })
                .bind(("127.0.0.1", 5500))
                .unwrap()
                .run(),
            )
            .unwrap();
    });
}
