use std::net::TcpListener;

use actix_web::{dev::Server, middleware::NormalizePath, web::{self, ServiceConfig}, App, HttpServer};
use tracing_actix_web::TracingLogger;

use crate::{config::AppConfig, scope::tts::tts_scope, TtsClient};

fn configure_server(config: &mut ServiceConfig) {
    config
        .service(tts_scope());
}

pub fn create_server(listener: TcpListener, config: AppConfig) -> anyhow::Result<Server> {
    let tts_client = web::Data::new(TtsClient::new(config.tts.base_url));

    let ref_audio_config = web::Data::new(config.ref_audio);

    let server = HttpServer::new(move || App::new()
        .wrap(TracingLogger::default())
        .wrap(NormalizePath::new(actix_web::middleware::TrailingSlash::MergeOnly))
        .configure(configure_server)
        .app_data(tts_client.clone())
        .app_data(ref_audio_config.clone())
        // TODO: add tts_config (ref_audio) app_data
    );

    Ok(server.listen(listener)?.run())
}
