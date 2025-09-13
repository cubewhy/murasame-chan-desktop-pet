use actix_web::{Responder, web};

use crate::{
    bus::{CommentEvent, InEvent},
    server::EventSender,
};

#[derive(serde::Deserialize)]
pub struct AddCommentModel {
    user: String,
    text: String,
}

pub async fn add_comment(
    payload: web::Json<AddCommentModel>,
    sender: web::Data<EventSender>,
) -> impl Responder {
    // TODO: nsfw filter
    let sender = &sender.0;
    sender
        .send(InEvent::Comment(CommentEvent {
            user: payload.user.to_owned(),
            text: payload.text.to_owned(),
        }))
        .await
        .unwrap(); // TODO: add error handling

    "ok" // TODO: response with json
}
