use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Mutex;
use std::thread::JoinHandle;

use anyhow::anyhow;

use super::router::ClientTransport;

/// Buffer size for outgoing messages. Prevents the browser-to-client routing
/// task from blocking on slow pipe writes.
const PIPE_WRITE_QUEUE_SIZE: usize = 4096;

/// Implements ClientTransport over stdin/stdout pipes.
pub struct PipeClientConn {
    inner: Mutex<PipeInner>,
    closed: AtomicBool,
}

struct PipeInner {
    tx: Option<SyncSender<String>>,
    handle: Option<JoinHandle<()>>,
}

/// Creates a PipeClientConn that writes protocol messages to `w`.
pub fn new_pipe_client_conn<W: Write + Send + 'static>(w: W) -> PipeClientConn {
    let (tx, rx) = sync_channel::<String>(PIPE_WRITE_QUEUE_SIZE);

    let handle = std::thread::spawn(move || {
        let mut writer = BufWriter::new(w);
        for msg in rx {
            if writer.write_all(msg.as_bytes()).is_err() {
                return;
            }
            if writer.write_all(b"\n").is_err() {
                return;
            }
            let _ = writer.flush();
        }
    });

    PipeClientConn {
        inner: Mutex::new(PipeInner {
            tx: Some(tx),
            handle: Some(handle),
        }),
        closed: AtomicBool::new(false),
    }
}

impl ClientTransport for PipeClientConn {
    /// Pipe mode supports exactly one client.
    fn id(&self) -> u64 {
        1
    }

    /// Queues a JSON message for writing to the pipe. Non-blocking: if the queue
    /// is full the message is dropped (matching Go's default-case drop).
    fn send(&self, msg: &str) -> anyhow::Result<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(anyhow!("pipe closed"));
        }
        let guard = self.inner.lock().unwrap();
        if let Some(tx) = &guard.tx {
            let _ = tx.try_send(msg.to_string());
        }
        Ok(())
    }

    /// Marks the pipe as closed and drains the write queue.
    fn close(&self) {
        if self.closed.swap(true, Ordering::SeqCst) {
            return;
        }
        let (tx, handle) = {
            let mut guard = self.inner.lock().unwrap();
            (guard.tx.take(), guard.handle.take())
        };
        drop(tx); // closes the channel so the writer thread exits
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}
