use std::{net::TcpListener, path::PathBuf};

use actix_web::{dev::Server, middleware::NormalizePath, web::{self, ServiceConfig}, App, HttpServer};
use tracing_actix_web::TracingLogger;

use crate::{config::RefAudioConfig, scope::tts::tts_scope, TtsClient};

fn configure_server(config: &mut ServiceConfig) {
    config
        .service(tts_scope());
}

pub fn create_server(listener: TcpListener) -> anyhow::Result<Server> {
    let tts_client = web::Data::new(TtsClient::new("http://127.0.0.1:9880")); // TODO:
                                                                                                                // read
                                                                                                                // from
                                                                                                                // config

    let ref_audio_config = web::Data::new(RefAudioConfig {
        text: "ふむ、おぬしが我輩のご主人か?".to_string(),
        path: std::env::current_dir()?.join("resources/ref_audio.ogg"),
    });

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
