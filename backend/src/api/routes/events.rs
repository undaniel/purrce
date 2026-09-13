use axum::{extract::State, response::sse::{Event, Sse}};
use futures_util::stream;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;

use crate::api::AppState;

pub async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.events.subscribe();
    let broadcast = BroadcastStream::new(rx).filter_map(|msg| {
        msg.ok().and_then(|ev| {
            serde_json::to_string(&ev).ok().map(|data| {
                Ok(Event::default().data(data))
            })
        })
    });

    // keepalive ping every 15s so proxies don't close idle connections
    let keepalive = stream::repeat(Ok(Event::default().comment("ping")))
        .throttle(Duration::from_secs(15));

    // merge: whichever produces next wins
    let merged = futures_util::stream::select(broadcast, keepalive);

    Sse::new(merged).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("ka"),
    )
}
