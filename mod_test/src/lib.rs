use futures::channel::oneshot::Receiver;
use futures::prelude::future;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;

use rustbot::prelude::*;
use rustbot::{span, spans};

#[no_mangle]
pub fn get_meta(meta: &mut dyn Meta) {
    meta.cmd(
        "test",
        Command::new(|ctx, args| {
            ctx.say(&format!("beep boop {}", ctx.perms()?))?;
            ctx.say(&format!("you passed: {}", args))?;
            ctx.reply(Message::Spans(spans!(
                "simple ",
                span!(Format::Bold; "bold"),
                " ",
                span!(Format::Italic; "italic"),
                " ",
                span!(Format::Underline; "underline"),
                " ",
                span!(Color::Red; "red"),
                " ",
                span!(Color::Yellow; "yellow"),
                " ",
                span!(Color::Green; "green"),
                " ",
                span!(Color::Red + Format::Bold + Format::Italic; "bold_italic_red"),
            )))
        }),
    );

    let unload = meta.on_unload_channel();

    meta.thread(Box::new(|| helloworldserver(unload)));
}

#[tokio::main]
async fn helloworldserver(unload: Receiver<()>) {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(hello_world)) });

    let server = Server::bind(&addr).serve(make_svc);

    future::select(unload, server).await;
}

async fn hello_world(_req: Request<Body>) -> std::result::Result<Response<Body>, Infallible> {
    Ok(Response::new("Hello, World".into()))
}
