use crate::layer::http_log_layer::HttpLogLayer;
use crate::with_event_from_span;
use serde_json::json;
use tokio::time::Instant;
use tracing::span::Attributes;
use tracing::{Event, Id, Subscriber, field};
use tracing_serde::AsSerde;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

struct Timings {
    idle: u64,
    busy: u64,
    last: Instant,
}

impl Timings {
    fn new() -> Self {
        Self {
            idle: 0,
            busy: 0,
            last: Instant::now(),
        }
    }
}

impl<S> Layer<S> for HttpLogLayer
where
    S: Subscriber
        + for<'span> tracing_subscriber::registry::LookupSpan<'span>
        + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
    Self: 'static,
{
    fn on_new_span(&self, _attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if extensions.get_mut::<Timings>().is_none() {
            extensions.insert(Timings::new());
        }

        with_event_from_span!(id, span, "message" = "new", |event| {
            drop(extensions);
            drop(span);
            self.on_event(&event, ctx);
        });
    }

    fn on_event(&self, event: &Event, _ctx: Context<S>) {
        println!("ON_EVENT");
        let log = json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "level": event.metadata().level().to_string(),
            "event": event.as_serde(),
        });
        let _ = self.event_tx.send(log);
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        let mut busy: Option<u64> = None;
        let mut idle: Option<u64> = None;

        if let Some(timings) = extensions.get_mut::<Timings>() {
            let now = Instant::now();
            timings.idle += (now - timings.last).as_nanos() as u64;
            timings.last = now;
            busy = Option::from(timings.busy);
            idle = Option::from(timings.idle);
        }
        with_event_from_span!(
            id,
            span,
            "message" = "enter",
            "time.busy" = busy,
            "time.idle" = idle,
            |event| {
                drop(extensions);
                drop(span);
                self.on_event(&event, ctx);
            }
        );
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        let mut busy: Option<u64> = None;
        let mut idle: Option<u64> = None;
        if let Some(timings) = extensions.get_mut::<Timings>() {
            let now = Instant::now();
            timings.busy += (now - timings.last).as_nanos() as u64;
            timings.last = now;
            busy = Option::from(timings.busy);
            idle = Option::from(timings.idle);
        }
        with_event_from_span!(
            id,
            span,
            "message" = "exit",
            "time.busy" = busy,
            "time.idle" = idle,
            |event| {
                drop(extensions);
                drop(span);
                self.on_event(&event, ctx);
            }
        );
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).expect("Span not found, this is a bug");
        let extensions = span.extensions();
        if let Some(timing) = extensions.get::<Timings>() {
            let Timings {
                busy,
                mut idle,
                last,
            } = *timing;
            idle += (Instant::now() - last).as_nanos() as u64;

            with_event_from_span!(
                id,
                span,
                "message" = "close",
                "time.busy" = busy,
                "time.idle" = idle,
                |event| {
                    drop(extensions);
                    drop(span);
                    self.on_event(&event, ctx);
                }
            );
        } else {
            with_event_from_span!(id, span, "message" = "close", |event| {
                drop(extensions);
                drop(span);
                self.on_event(&event, ctx);
            });
        }
    }
}
