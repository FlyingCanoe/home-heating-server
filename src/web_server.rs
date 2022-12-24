use std::sync::Arc;
use std::sync::Mutex;

use actix_files::Files;
use actix_web::middleware::Logger;
use actix_web::{http, web, App, HttpResponse, HttpServer};
use tokio::sync::mpsc;
use tokio::sync::watch;

use crate::control_server::ControlState;
use crate::db::{DbRequest, DbResponse};

#[actix_web::get("/rest-api/get-error")]
async fn serve_error(
    control_state: web::Data<watch::Receiver<ControlState>>,
) -> Result<HttpResponse, http::Error> {
    Ok(HttpResponse::Ok().json(control_state.borrow().error.clone()))
}

#[actix_web::get("/rest-api/thermometer-list")]
async fn thermometer_list(
    control_state: web::Data<watch::Receiver<ControlState>>,
) -> Result<HttpResponse, http::Error> {
    let thermometer_list: Vec<_> = control_state
        .borrow()
        .thermometer_list
        .clone()
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    Ok(HttpResponse::Ok().json(thermometer_list))
}

#[actix_web::get("/rest-api/thermometer-status/{name}")]
async fn thermometer_status(
    control_state: web::Data<watch::Receiver<ControlState>>,
    name: web::Path<String>,
) -> Result<HttpResponse, http::Error> {
    if let Some(thermometer) = control_state
        .borrow()
        .thermometer_list
        .get(&name.to_string())
    {
        Ok(HttpResponse::Ok().json(thermometer.clone()))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
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
    Ok(HttpResponse::Ok().json(history))
}

pub(crate) fn start_web_server(
    control_state: watch::Receiver<ControlState>,
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
                        .wrap(Logger::new("%r %D"))
                        .app_data(web::Data::new(db_conn.clone()))
                        .app_data(web::Data::new(control_state.clone()))
                        .app_data(web::Data::new(control_tx.clone()))
                        .service(serve_error)
                        .service(thermometer_history)
                        .service(change_thermometer_target)
                        .service(thermometer_status)
                        .service(thermometer_list)
                        .service(Files::new("", "webapp/dist").index_file("index.html"))
                })
                .bind(("127.0.0.1", 5500))
                .unwrap()
                .run(),
            )
            .unwrap();
    });
}
