//! Keeps the game's recent stdout and stderr, so tools connected through the
//! control endpoint can read them (`logs.get`, MCP `read_logs`), also for
//! games started from a terminal or an IDE rather than by the tool.
//!
//! While a control endpoint is open, standard output and error point at
//! pipes (file descriptors 1 and 2 on Unix; the standard handles and the C
//! runtime's descriptors on Windows). A thread per stream copies everything
//! to where it went before (the terminal still shows it all) and keeps the
//! last [`MAX_LINES`] lines. That covers Rust's `println!`, C and C++
//! `printf`/`std::cout`, and panic messages.

// Only filled by a capture; on other platforms the ring stays empty.
#![cfg_attr(not(any(unix, windows)), allow(dead_code))]

use std::collections::VecDeque;
use std::sync::Mutex;

use serde_json::{Value, json};

/// Lines kept, oldest dropped first.
pub(crate) const MAX_LINES: usize = 5_000;
/// Longer lines are cut, so one huge line can't fill the memory.
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
        return Err(if cfg!(any(unix, windows)) {
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

#[cfg(any(unix, windows))]
pub(crate) use capture::{Capture, capturing};

#[cfg(not(any(unix, windows)))]
pub(crate) struct Capture;

#[cfg(not(any(unix, windows)))]
impl Capture {
    pub(crate) fn start() -> Option<Self> {
        None
    }
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn capturing() -> bool {
    false
}

#[cfg(any(unix, windows))]
mod capture {
    use std::fs::File;
    use std::io::{PipeReader, Read, Write};
    use std::sync::Mutex;
    use std::sync::mpsc::{self, Receiver};
    use std::thread;
    use std::time::Duration;

    #[cfg(unix)]
    use super::unix as sys;
    #[cfg(windows)]
    use super::windows as sys;

    /// How long a restore waits for the copy threads to pass on the last
    /// output. Longer only if a child process still holds the pipe.
    const DRAIN_WAIT: Duration = Duration::from_millis(500);

    /// Standard output or standard error.
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub(super) enum Std {
        Out,
        Err,
    }

    /// One redirected stream: how to put it back, and the thread copying.
    struct Stream {
        which: Std,
        saved: sys::Saved,
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
                for which in [Std::Out, Std::Err] {
                    match redirect(which) {
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

    /// Points `which` at a new pipe and starts the thread that copies from
    /// it to where the stream went before.
    fn redirect(which: Std) -> std::io::Result<Stream> {
        flush(which);
        let (saved, reader, out) = sys::redirect(which)?;
        let done = spawn_copier(which, reader, out)?;
        Ok(Stream { which, saved, done })
    }

    /// Copies everything from `reader` to `out` and keeps it line by line,
    /// until the pipe's write end is closed.
    fn spawn_copier(which: Std, mut reader: PipeReader, mut out: File) -> std::io::Result<Receiver<()>> {
        let (done_tx, done) = mpsc::channel();
        let name = if which == Std::Out {
            "crabby-logs-out"
        } else {
            "crabby-logs-err"
        };
        thread::Builder::new().name(name.into()).spawn(move || {
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
        Ok(done)
    }

    /// Puts every stream back where it was and lets the threads finish
    /// passing on what's left.
    fn undo(streams: &mut Vec<Stream>, flush_std: bool) {
        for s in streams.iter_mut() {
            if flush_std {
                flush(s.which);
            }
            sys::restore(s.which, &mut s.saved);
        }
        for s in streams.drain(..) {
            let _ = s.done.recv_timeout(DRAIN_WAIT);
        }
    }

    fn flush(which: Std) {
        if which == Std::Out {
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

/// Unix: file descriptors 1 and 2 point at the pipes (`dup2`), which covers
/// Rust, C and C++ output alike.
#[cfg(unix)]
mod unix {
    use std::fs::File;
    use std::io::PipeReader;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

    use super::capture::Std;

    /// The descriptor the stream used before.
    pub(super) struct Saved(OwnedFd);

    fn fd(which: Std) -> RawFd {
        if which == Std::Out { 1 } else { 2 }
    }

    /// Returns what to restore, the pipe's read end, and where the output went.
    pub(super) fn redirect(which: Std) -> std::io::Result<(Saved, PipeReader, File)> {
        let fd = fd(which);
        // SAFETY: dup of a valid (or invalid, then -1) descriptor number.
        let copy = unsafe { libc::dup(fd) };
        if copy < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: `copy` is a fresh descriptor that nothing else owns.
        let original = unsafe { OwnedFd::from_raw_fd(copy) };
        let (reader, writer) = std::io::pipe()?;
        let out = File::from(original.try_clone()?);
        // SAFETY: both descriptors are valid; fd now refers to the pipe.
        if unsafe { libc::dup2(writer.as_raw_fd(), fd) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // `writer` closes here: `fd` is the pipe's only write end, so the
        // copy thread sees the end of the stream once `fd` is restored.
        Ok((Saved(original), reader, out))
    }

    pub(super) fn restore(which: Std, saved: &mut Saved) {
        // SAFETY: valid descriptors; closes this fd's pipe write end.
        unsafe { libc::dup2(saved.0.as_raw_fd(), fd(which)) };
    }
}

/// Windows: Rust writes to the process's standard handles and C and C++
/// to the C runtime's descriptors 1 and 2, so both point at the pipes.
#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    use std::fs::File;
    use std::io::{PipeReader, PipeWriter};
    use std::os::windows::io::{AsRawHandle, BorrowedHandle, IntoRawHandle};

    use super::capture::Std;

    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetStdHandle(id: u32) -> *mut c_void;
        fn SetStdHandle(id: u32, handle: *mut c_void) -> i32;
    }

    /// Where the stream went before, and the pipe end the standard handle
    /// now refers to (kept open until the restore; dropping it ends the copy).
    pub(super) struct Saved {
        handle: *mut c_void,
        /// The C runtime's descriptor before, if it had one open.
        crt: Option<i32>,
        writer: Option<PipeWriter>,
    }

    // SAFETY: a process-wide handle value, only passed back to SetStdHandle.
    unsafe impl Send for Saved {}

    fn ids(which: Std) -> (u32, i32) {
        if which == Std::Out {
            (STD_OUTPUT_HANDLE, 1)
        } else {
            (STD_ERROR_HANDLE, 2)
        }
    }

    pub(super) fn redirect(which: Std) -> std::io::Result<(Saved, PipeReader, File)> {
        let (id, crt_fd) = ids(which);
        // SAFETY: plain query of the process's standard handle.
        let handle = unsafe { GetStdHandle(id) };
        if handle.is_null() || handle as isize == -1 {
            // A GUI program without a console: nothing to capture.
            return Err(std::io::Error::other("no standard handle"));
        }
        // SAFETY: the standard handle stays open while we borrow it here.
        let out = unsafe { BorrowedHandle::borrow_raw(handle) }.try_clone_to_owned()?;
        let (reader, writer) = std::io::pipe()?;

        // SAFETY: CRT calls on descriptor numbers; failures return -1.
        let crt = unsafe {
            let before = libc::dup(crt_fd);
            if before >= 0 {
                let pipe_fd = libc::open_osfhandle(writer.try_clone()?.into_raw_handle() as isize, libc::O_BINARY);
                if pipe_fd >= 0 {
                    libc::dup2(pipe_fd, crt_fd);
                    libc::close(pipe_fd);
                }
                Some(before)
            } else {
                None
            }
        };
        // SAFETY: `writer` stays open (in `Saved`) while it's the standard handle.
        if unsafe { SetStdHandle(id, writer.as_raw_handle()) } == 0 {
            let err = std::io::Error::last_os_error();
            let mut saved = Saved {
                handle,
                crt,
                writer: Some(writer),
            };
            restore(which, &mut saved);
            return Err(err);
        }
        Ok((
            Saved {
                handle,
                crt,
                writer: Some(writer),
            },
            reader,
            File::from(out),
        ))
    }

    pub(super) fn restore(which: Std, saved: &mut Saved) {
        let (id, crt_fd) = ids(which);
        // SAFETY: puts back the handle and descriptor saved in `redirect`.
        unsafe {
            SetStdHandle(id, saved.handle);
            if let Some(before) = saved.crt.take() {
                libc::dup2(before, crt_fd);
                libc::close(before);
            }
        }
        // The last write end: the copy thread now sees the end of the stream.
        saved.writer = None;
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
