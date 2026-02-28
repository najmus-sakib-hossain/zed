//! Actix-web WebSocket server

use actix::{Actor, StreamHandler};
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, web};
use actix_web_actors::ws;
use anyhow::Result;

pub struct WsSession;

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

async fn ws_index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(WsSession {}, &req, stream)
}

pub async fn start_ws_server(port: u16) -> Result<()> {
    HttpServer::new(|| App::new().route("/ws", web::get().to(ws_index)))
        .bind(("127.0.0.1", port))?
        .run()
        .await?;

    Ok(())
}
