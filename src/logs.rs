//! Keeps the game's recent stdout and stderr, so tools connected through the
//! control endpoint can read them (`logs.get`, MCP `read_logs`), also for
//! games started from a terminal or an IDE rather than by the tool.
//!
//! While a control endpoint is open, file descriptors 1 and 2 point at pipes.
//! A thread per stream copies everything to where it went before (the
//! terminal still shows it all) and keeps the last [`MAX_LINES`] lines. That
//! covers Rust's `println!`, C and C++ `printf`/`std::cout`, and panic
//! messages. Unix only for now; elsewhere `logs.get` says so.

use std::collections::VecDeque;
use std::sync::Mutex;

use serde_json::{Value, json};

/// Lines kept, oldest dropped first.
pub(crate) const MAX_LINES: usize = 5_000;
/// Longer lines are cut, so one huge line can't fill the memory.
#[cfg_attr(not(unix), allow(dead_code))]
const MAX_LINE_BYTES: usize = 4_096;

/// The kept lines. `next` numbers lines from the start of the capture, so a
/// reader can ask for what's new since its last read.
struct Ring {
    lines: VecDeque<String>,
    next: u64,
}

static RING: Mutex<Ring> = Mutex::new(Ring {
    lines: VecDeque::new(),
    next: 0,
});

#[cfg_attr(not(unix), allow(dead_code))]
fn push_line(bytes: &[u8]) {
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    let cut = &bytes[..bytes.len().min(MAX_LINE_BYTES)];
    let line = String::from_utf8_lossy(cut).into_owned();
    let mut ring = RING.lock().unwrap_or_else(|e| e.into_inner());
    if ring.lines.len() == MAX_LINES {
        ring.lines.pop_front();
    }
    ring.lines.push_back(line);
    ring.next += 1;
}

/// The `logs.get` answer. Without `since`: the newest `max` lines. With it:
/// up to `max` lines from line number `since` on, oldest first. `next` is the
/// `since` for the following read; `missed` counts lines dropped (the ring
/// is full) before they could be read.
pub(crate) fn get(since: Option<u64>, max: usize) -> Result<Value, String> {
    if !capturing() {
        return Err(if cfg!(unix) {
            "this game's output isn't captured (the pipes couldn't be set up); check the terminal it runs in".into()
        } else {
            "game output isn't captured on this platform yet; check the terminal the game runs in".into()
        });
    }
    let ring = RING.lock().unwrap_or_else(|e| e.into_inner());
    let first = ring.next - ring.lines.len() as u64;
    let (from, missed) = match since {
        None => (ring.next.saturating_sub(max as u64).max(first), 0),
        Some(s) => (s.clamp(first, ring.next), first.saturating_sub(s)),
    };
    let lines: Vec<&str> = ring
        .lines
        .iter()
        .skip((from - first) as usize)
        .take(max)
        .map(String::as_str)
        .collect();
    let next = from + lines.len() as u64;
    Ok(json!({ "lines": lines, "next": next, "missed": missed }))
}

#[cfg(unix)]
pub(crate) use unix::{Capture, capturing};

#[cfg(not(unix))]
pub(crate) struct Capture;

#[cfg(not(unix))]
impl Capture {
    pub(crate) fn start() -> Option<Self> {
        None
    }
}

#[cfg(not(unix))]
pub(crate) fn capturing() -> bool {
    false
}

#[cfg(unix)]
mod unix {
    use std::fs::File;
    use std::io::{Read, Write};
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
    use std::sync::Mutex;
    use std::sync::mpsc::{self, Receiver};
    use std::thread;
    use std::time::Duration;

    /// How long a restore waits for the copy threads to pass on the last
    /// output. Longer only if a child process still holds the pipe.
    const DRAIN_WAIT: Duration = Duration::from_millis(500);

    /// One redirected stream: where it went before, and the thread copying.
    struct Stream {
        fd: RawFd,
        original: OwnedFd,
        done: Receiver<()>,
    }

    /// The process-wide capture, shared by every open endpoint: started by
    /// the first, undone when the last closes (or the process exits).
    struct State {
        users: usize,
        streams: Vec<Stream>,
    }

    static STATE: Mutex<State> = Mutex::new(State {
        users: 0,
        streams: Vec::new(),
    });

    fn state() -> std::sync::MutexGuard<'static, State> {
        STATE.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn capturing() -> bool {
        !state().streams.is_empty()
    }

    /// Keeps the capture on while alive.
    pub(crate) struct Capture(());

    impl Capture {
        /// Starts capturing (or joins the running capture). `None` if the
        /// pipes couldn't be set up; output then just isn't kept.
        pub(crate) fn start() -> Option<Self> {
            let mut s = state();
            if s.users == 0 {
                let mut streams = Vec::new();
                for fd in [1, 2] {
                    match redirect(fd) {
                        Ok(stream) => streams.push(stream),
                        Err(_) => {
                            undo(&mut streams, true);
                            return None;
                        }
                    }
                }
                s.streams = streams;
                register_exit_hook();
            }
            s.users += 1;
            Some(Capture(()))
        }
    }

    impl Drop for Capture {
        fn drop(&mut self) {
            let mut s = state();
            s.users -= 1;
            if s.users == 0 {
                undo(&mut s.streams, true);
            }
        }
    }

    /// Points `fd` at a new pipe and starts the thread that copies from it
    /// to where `fd` pointed before.
    fn redirect(fd: RawFd) -> std::io::Result<Stream> {
        // SAFETY: dup of a valid (or invalid, then -1) descriptor number.
        let copy = unsafe { libc::dup(fd) };
        if copy < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: `copy` is a fresh descriptor that nothing else owns.
        let original = unsafe { OwnedFd::from_raw_fd(copy) };
        let (mut reader, writer) = std::io::pipe()?;
        let mut out = File::from(original.try_clone()?);
        let (done_tx, done) = mpsc::channel();
        thread::Builder::new()
            .name(format!("crabby-logs-{fd}"))
            .spawn(move || {
                let mut buf = [0u8; 8192];
                let mut partial = Vec::new();
                while let Ok(n) = reader.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    let _ = out.write_all(&buf[..n]);
                    partial.extend_from_slice(&buf[..n]);
                    let mut start = 0;
                    while let Some(i) = partial[start..].iter().position(|&b| b == b'\n') {
                        super::push_line(&partial[start..start + i]);
                        start += i + 1;
                    }
                    partial.drain(..start);
                    if partial.len() > super::MAX_LINE_BYTES {
                        super::push_line(&partial);
                        partial.clear();
                    }
                }
                if !partial.is_empty() {
                    super::push_line(&partial);
                }
                let _ = done_tx.send(());
            })?;
        flush(fd);
        // SAFETY: both descriptors are valid; fd now refers to the pipe.
        if unsafe { libc::dup2(writer.as_raw_fd(), fd) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // `writer` closes here: `fd` is the pipe's only write end, so the
        // thread sees the end of the stream once `fd` is restored.
        Ok(Stream { fd, original, done })
    }

    /// Puts every stream back where it was and lets the threads finish
    /// passing on what's left.
    fn undo(streams: &mut Vec<Stream>, flush_std: bool) {
        for s in streams.iter() {
            if flush_std {
                flush(s.fd);
            }
            // SAFETY: valid descriptors; closes this fd's pipe write end.
            unsafe { libc::dup2(s.original.as_raw_fd(), s.fd) };
        }
        for s in streams.drain(..) {
            let _ = s.done.recv_timeout(DRAIN_WAIT);
        }
    }

    fn flush(fd: RawFd) {
        if fd == 1 {
            let _ = std::io::stdout().flush();
        } else {
            let _ = std::io::stderr().flush();
        }
    }

    /// `std::process::exit` skips destructors: restore at exit too, so the
    /// last lines a game prints still reach the terminal.
    fn register_exit_hook() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        extern "C" fn restore_at_exit() {
            // try_lock: never wait at exit (a thread may hold it forever).
            // No flushing either: another thread may hold Rust's stdout lock.
            if let Ok(mut s) = STATE.try_lock() {
                undo(&mut s.streams, false);
            }
        }
        // SAFETY: registers a plain function with no captured state.
        ONCE.call_once(|| unsafe {
            libc::atexit(restore_at_exit);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_cut_and_numbered() {
        // The ring is global; only this test pushes without a capture.
        let before = RING.lock().unwrap().next;
        push_line(b"hello\r");
        push_line(&[b'x'; MAX_LINE_BYTES * 2]);
        let ring = RING.lock().unwrap();
        assert_eq!(ring.next, before + 2);
        let n = ring.lines.len();
        assert_eq!(ring.lines[n - 2], "hello");
        assert_eq!(ring.lines[n - 1].len(), MAX_LINE_BYTES);
    }
}
