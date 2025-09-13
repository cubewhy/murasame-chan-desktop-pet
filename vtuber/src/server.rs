use std::net::TcpListener;

use actix_web::{
    App, HttpServer,
    dev::Server,
    web::{self, ServiceConfig},
};
use tokio::sync::mpsc;

use crate::{bus::InEvent, scope::comments::comments_scope};

fn config_server(config: &mut ServiceConfig) {
    config.service(comments_scope());
}

pub struct EventSender(pub mpsc::Sender<InEvent>);

pub fn create_server(
    listener: TcpListener,
    in_tx: mpsc::Sender<InEvent>,
) -> anyhow::Result<Server> {
    let event_sender = web::Data::new(EventSender(in_tx));
    let server = HttpServer::new(move || {
        App::new()
            .configure(config_server)
            .app_data(event_sender.clone())
    });

    Ok(server.listen(listener)?.run())
}
