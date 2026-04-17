use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use serde::Serialize;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::mpsc;

use super::models::ProgressEvent;

/// Trait for reading progress state from a shared mutex.
pub trait ProgressState: Send + 'static {
    fn current(&self) -> usize;
    fn total(&self) -> usize;
    fn finished(&self) -> bool;
}

impl ProgressState for fsrs::CombinedProgressState {
    fn current(&self) -> usize {
        self.current()
    }
    fn total(&self) -> usize {
        self.total()
    }
    fn finished(&self) -> bool {
        self.finished()
    }
}

/// Simple progress state that can be written to from a callback.
pub struct SimpleProgress {
    pub current: usize,
    pub total: usize,
}

impl ProgressState for SimpleProgress {
    fn current(&self) -> usize {
        self.current
    }
    fn total(&self) -> usize {
        self.total
    }
    fn finished(&self) -> bool {
        false
    }
}

#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
enum SseMessage<T: Serialize> {
    #[serde(rename = "progress")]
    Progress(ProgressEvent),
    #[serde(rename = "result")]
    Result(T),
    #[serde(rename = "error")]
    Error { error: String },
}

struct SseStream<T: Serialize> {
    rx: mpsc::Receiver<SseMessage<T>>,
}

impl<T: Serialize> Stream for SseStream<T> {
    type Item = Result<Event, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(msg)) => {
                let event_type = match &msg {
                    SseMessage::Progress(_) => "progress",
                    SseMessage::Result(_) => "result",
                    SseMessage::Error { .. } => "error",
                };
                let data = serde_json::to_string(&msg).unwrap_or_default();
                Poll::Ready(Some(Ok(Event::default().event(event_type).data(data))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Create an SSE stream for any blocking task with progress reporting.
///
/// The stream emits `progress` events periodically, then a `result` or `error` event,
/// and closes immediately after the task completes.
pub fn progress_stream<P, T, E, F>(
    progress: Arc<Mutex<P>>,
    task: F,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>>
where
    P: ProgressState,
    T: Serialize + Send + 'static,
    E: std::fmt::Display + Send + 'static,
    F: FnOnce() -> Result<T, E> + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<SseMessage<T>>(32);
    let done = Arc::new(tokio::sync::Notify::new());

    // Spawn the blocking task
    let tx_result = tx.clone();
    let done_signal = Arc::clone(&done);
    tokio::task::spawn_blocking(move || {
        let msg = match task() {
            Ok(result) => SseMessage::Result(result),
            Err(e) => SseMessage::Error {
                error: e.to_string(),
            },
        };
        let _ = tx_result.blocking_send(msg);
        done_signal.notify_one();
    });

    // Spawn progress reporter — stops immediately when task is done
    let progress_ref = progress;
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(200));
        loop {
            tokio::select! {
                _ = tick.tick() => {
                    let (current, total, finished) = {
                        let state = progress_ref.lock().unwrap();
                        (state.current(), state.total(), state.finished())
                    };

                    let _ = tx
                        .send(SseMessage::Progress(ProgressEvent {
                            current,
                            total,
                            finished,
                        }))
                        .await;

                    if finished {
                        break;
                    }
                }
                _ = done.notified() => {
                    break;
                }
            }
        }
    });

    Sse::new(SseStream { rx }).keep_alive(KeepAlive::default())
}
