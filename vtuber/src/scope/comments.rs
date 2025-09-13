use actix_web::{Scope, web};

use crate::handler::comments::add_comment;

pub fn comments_scope() -> Scope {
    web::scope("comments").route("add", web::post().to(add_comment))
}
