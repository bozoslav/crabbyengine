//! GPU frame timing with timestamp queries.
//!
//! The sprite pass writes a timestamp when it starts and when it ends. The
//! results come back through a small ring of readback buffers, so reading
//! them never waits for the GPU: a frame's time shows up one to three frames
//! later. If every buffer is still in flight, that frame isn't timed.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

const SLOTS: usize = 3;
const FREE: u8 = 0;
const MAPPING: u8 = 1;
const READY: u8 = 2;

struct Slot {
    buffer: wgpu::Buffer,
    state: Arc<AtomicU8>,
}

pub(crate) struct GpuTimer {
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    slots: [Slot; SLOTS],
    /// Nanoseconds per timestamp tick.
    period: f64,
    /// The slot this frame writes to, if one was free.
    current: Option<usize>,
    next: usize,
    /// Smoothed GPU time of the sprite pass, in milliseconds.
    pub smoothed_ms: Option<f32>,
}

impl GpuTimer {
    /// `None` if the GPU can't do timestamp queries.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("frame timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("timestamp resolve"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let slot = || Slot {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("timestamp readback"),
                size: 16,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            state: Arc::new(AtomicU8::new(FREE)),
        };
        Some(Self {
            queries,
            resolve,
            slots: [slot(), slot(), slot()],
            period: f64::from(queue.get_timestamp_period()),
            current: None,
            next: 0,
            smoothed_ms: None,
        })
    }

    /// Picks up finished readbacks. Call once per frame, before `begin`.
    pub fn collect(&mut self, device: &wgpu::Device) {
        let _ = device.poll(wgpu::PollType::Poll);
        for slot in &self.slots {
            if slot.state.load(Ordering::Acquire) != READY {
                continue;
            }
            let ticks = {
                match slot.buffer.get_mapped_range(..) {
                    Ok(view) => {
                        let t: &[u64] = bytemuck::cast_slice(&view);
                        Some(t[1].saturating_sub(t[0]))
                    }
                    Err(_) => None,
                }
            };
            slot.buffer.unmap();
            slot.state.store(FREE, Ordering::Release);
            let Some(ticks) = ticks else { continue };
            let ms = (ticks as f64 * self.period / 1e6) as f32;
            // A bogus sample (clock reset, overflow) would ruin the average.
            if ms.is_finite() && ms < 10_000.0 {
                self.smoothed_ms = Some(match self.smoothed_ms {
                    None => ms,
                    Some(avg) => avg + (ms - avg) * 0.1,
                });
            }
        }
    }

    /// The timestamp writes for this frame's pass, if a readback slot is free.
    pub fn begin(&mut self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let i = self.next;
        self.current = (self.slots[i].state.load(Ordering::Acquire) == FREE).then_some(i);
        self.current?;
        self.next = (i + 1) % SLOTS;
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: Some(0),
            end_of_pass_write_index: Some(1),
        })
    }

    /// Copies the timestamps out. Call after the pass, before submitting.
    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        if let Some(i) = self.current {
            encoder.resolve_query_set(&self.queries, 0..2, &self.resolve, 0);
            encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.slots[i].buffer, 0, 16);
        }
    }

    /// Starts reading this frame's timestamps back. Call after submitting.
    pub fn submitted(&mut self) {
        if let Some(i) = self.current.take() {
            let slot = &self.slots[i];
            slot.state.store(MAPPING, Ordering::Release);
            let state = slot.state.clone();
            slot.buffer.map_async(wgpu::MapMode::Read, .., move |result| {
                state.store(if result.is_ok() { READY } else { FREE }, Ordering::Release);
            });
        }
    }
}
