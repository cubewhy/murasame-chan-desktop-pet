use std::net::TcpListener;

use actix_web::{dev::Server, web::{self, ServiceConfig}, App, HttpServer};

use crate::{scope::tts::tts_scope, TtsClient};

fn configure_server(config: &mut ServiceConfig) {
    config.service(tts_scope());
}

pub fn create_server(listener: TcpListener) -> anyhow::Result<Server> {
    let tts_client = web::Data::new(TtsClient::new("http://127.0.0.1:9880")); // TODO:
                                                                                                                // read
                                                                                                                // from
                                                                                                                // config

    let server = HttpServer::new(move || App::new()
        .configure(configure_server)
        .app_data(tts_client.clone())
        // TODO: add tts_config (ref_audio) app_data
    );

    Ok(server.listen(listener)?.run())
}
