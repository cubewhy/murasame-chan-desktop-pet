use actix_web::{web, Scope};

use crate::handler;

pub fn tts_scope() -> Scope {
    web::scope("/tts")
        .route("generate", web::get().to(handler::tts::generate_tts))
}
