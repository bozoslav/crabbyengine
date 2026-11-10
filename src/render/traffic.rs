//! Counts what the renderer sends to the GPU: bytes written into buffers
//! and textures, by kind, and buffers created. Every upload and buffer
//! creation in the renderer goes through these functions; the renderer
//! takes the counts once per drawn frame into `RenderStats::uploads`.
//!
//! The counts live in a thread-local: a renderer prepares, uploads and
//! takes them on one thread, and renderers on other threads (tests run
//! side by side) don't mix into each other's counts.

use std::cell::Cell;

use wgpu::util::DeviceExt;

use super::GpuUploads;

/// What an upload is for: the field of [`GpuUploads`] it counts into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(feature = "3d"), allow(dead_code))]
pub(crate) enum Upload {
    /// Per-frame 3D model records (transforms and colors).
    Instances,
    /// 2D sprite records.
    Sprites,
    /// Persistent instance batches and their culling data.
    StaticBatches,
    /// Small per-frame uniforms: cameras, post-processing, effect settings.
    Uniforms,
    /// Light and decal grids.
    Lights,
    /// Material parameters.
    Materials,
    /// Skinning: joint matrices, weights, posed vertices.
    Skins,
    /// Mesh vertices and indices, and game-made GPU buffers.
    Meshes,
    /// Texture pixels (images, probes).
    Textures,
    /// Everything else (particles, 3D text, occlusion grids, ...).
    Other,
}

thread_local! {
    static COUNTS: Cell<GpuUploads> = const { Cell::new(GpuUploads::ZERO) };
}

fn count(f: impl FnOnce(&mut GpuUploads)) {
    COUNTS.with(|c| {
        let mut counts = c.get();
        f(&mut counts);
        c.set(counts);
    });
}

fn add(kind: Upload, bytes: u64) {
    count(|c| {
        let field = match kind {
            Upload::Instances => &mut c.instances,
            Upload::Sprites => &mut c.sprites,
            Upload::StaticBatches => &mut c.static_batches,
            Upload::Uniforms => &mut c.uniforms,
            Upload::Lights => &mut c.lights,
            Upload::Materials => &mut c.materials,
            Upload::Skins => &mut c.skins,
            Upload::Meshes => &mut c.meshes,
            Upload::Textures => &mut c.textures,
            Upload::Other => &mut c.other,
        };
        *field += bytes;
    });
}

fn created(size: u64) {
    count(|c| {
        c.buffers_created += 1;
        c.buffer_bytes_created += size;
    });
}

/// Counted uploads and buffer creations: `device.new_buffer(&desc)` for
/// `create_buffer`, `device.uploading(kind).create_buffer_init(&desc)`,
/// and `queue.uploading(kind).write_buffer(..)` / `.write_texture(..)`.
pub(crate) trait Counted {
    /// This device or queue, counting what it uploads as `kind`.
    fn uploading(&self, kind: Upload) -> Uploading<'_, Self> {
        Uploading { to: self, kind }
    }
}
impl Counted for wgpu::Device {}
impl Counted for wgpu::Queue {}

/// A device or queue whose uploads count as one kind.
pub(crate) struct Uploading<'a, T: ?Sized> {
    to: &'a T,
    kind: Upload,
}

impl Uploading<'_, wgpu::Queue> {
    /// `Queue::write_buffer`, counted.
    pub fn write_buffer(&self, buffer: &wgpu::Buffer, offset: u64, data: &[u8]) {
        add(self.kind, data.len() as u64);
        self.to.write_buffer(buffer, offset, data);
    }
    /// `Queue::write_texture`, counted.
    pub fn write_texture(
        &self,
        texture: wgpu::TexelCopyTextureInfo<'_>,
        data: &[u8],
        layout: wgpu::TexelCopyBufferLayout,
        size: wgpu::Extent3d,
    ) {
        add(self.kind, data.len() as u64);
        self.to.write_texture(texture, data, layout, size);
    }
}

#[cfg_attr(not(feature = "3d"), allow(dead_code))]
impl Uploading<'_, wgpu::Device> {
    /// `DeviceExt::create_buffer_init`, counted: a buffer created, and its
    /// contents uploaded.
    pub fn create_buffer_init(&self, desc: &wgpu::util::BufferInitDescriptor<'_>) -> wgpu::Buffer {
        let buffer = self.to.create_buffer_init(desc);
        created(buffer.size());
        add(self.kind, desc.contents.len() as u64);
        buffer
    }
}

/// `Device::create_buffer`, counted.
pub(crate) trait NewBuffer {
    fn new_buffer(&self, desc: &wgpu::BufferDescriptor<'_>) -> wgpu::Buffer;
}
impl NewBuffer for wgpu::Device {
    fn new_buffer(&self, desc: &wgpu::BufferDescriptor<'_>) -> wgpu::Buffer {
        created(desc.size);
        self.create_buffer(desc)
    }
}

/// What a GPU buffer (or texture) holds, kept on the CPU, so a frame
/// writes only the parts that changed since the last.
///
/// Forget it whenever the GPU side is replaced (a buffer outgrown and made
/// anew), or the next writes would be skipped against the old contents.
#[derive(Default)]
#[cfg_attr(not(feature = "3d"), allow(dead_code))]
pub(crate) struct Mirror {
    bytes: Vec<u8>,
}

#[cfg_attr(not(feature = "3d"), allow(dead_code))]
impl Mirror {
    /// The GPU side's contents are unknown (a new buffer): write everything next.
    pub fn forget(&mut self) {
        self.bytes.clear();
    }

    /// Brings bytes `offset..offset + new.len()` up to `new`: calls
    /// `write(at, bytes)` for each run of changed `page`-sized pages (one
    /// call per run of neighbouring ones). Writes past what is known are
    /// always made. `offset` and `page` keep `wgpu`'s 4-byte copy alignment
    /// when they and `new.len()` are multiples of 4.
    pub fn sync(&mut self, offset: usize, new: &[u8], page: usize, write: impl FnMut(usize, &[u8])) {
        self.sync_bridged(offset, new, page, 0, write);
    }

    /// Like [`sync`](Self::sync), but changed pages with up to `bridge`
    /// unchanged ones between them go in one write (the unchanged along
    /// with them): small pages then cost no more writes than large ones,
    /// while a lone change still uploads one page.
    pub fn sync_bridged(
        &mut self,
        offset: usize,
        new: &[u8],
        page: usize,
        bridge: usize,
        mut write: impl FnMut(usize, &[u8]),
    ) {
        let page = page.max(1);
        if offset > self.bytes.len() {
            // A gap of unknown bytes: only what's contiguous is mirrored.
            write(offset, new);
            return;
        }
        let end = offset + new.len();
        let known = self.bytes.len().min(end) - offset;
        // The run of pages to write: where it starts, and where its last
        // changed page ends.
        let mut run: Option<(usize, usize)> = None;
        let mut at = 0;
        while at < new.len() {
            let to = (at + page).min(new.len());
            let same = to <= known && self.bytes[offset + at..offset + to] == new[at..to];
            if !same {
                run = match run {
                    Some((start, last)) if at - last <= bridge * page => Some((start, to)),
                    Some((start, last)) => {
                        write(offset + start, &new[start..last]);
                        Some((at, to))
                    }
                    None => Some((at, to)),
                };
            }
            at = to;
        }
        if let Some((start, last)) = run {
            write(offset + start, &new[start..last]);
        }
        if end > self.bytes.len() {
            self.bytes.resize(end, 0);
        }
        self.bytes[offset..end].copy_from_slice(new);
    }
}

/// The counts since the last call on this thread, which are then reset.
pub(crate) fn take() -> GpuUploads {
    COUNTS.with(|c| c.replace(GpuUploads::ZERO))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_add_up_by_kind_and_reset_when_taken() {
        take();
        add(Upload::Instances, 64);
        add(Upload::Instances, 16);
        add(Upload::Lights, 8);
        created(256);
        let c = take();
        assert_eq!((c.instances, c.lights, c.bytes()), (80, 8, 88));
        assert_eq!((c.buffers_created, c.buffer_bytes_created), (1, 256));
        assert_eq!(take(), GpuUploads::default());
    }

    fn writes(mirror: &mut Mirror, offset: usize, new: &[u8]) -> Vec<(usize, Vec<u8>)> {
        let mut out = Vec::new();
        mirror.sync(offset, new, 4, |at, b| out.push((at, b.to_vec())));
        out
    }

    #[test]
    fn mirrors_write_only_changed_pages() {
        let mut m = Mirror::default();
        let first: Vec<u8> = (0..16).collect();
        assert_eq!(writes(&mut m, 0, &first), [(0, first.clone())], "unknown: all of it");
        assert!(writes(&mut m, 0, &first).is_empty(), "unchanged: nothing");
        let mut second = first.clone();
        second[5] = 99;
        second[6] = 98;
        second[13] = 97;
        assert_eq!(
            writes(&mut m, 0, &second),
            [(4, second[4..8].to_vec()), (12, second[12..16].to_vec())]
        );
        // Neighbouring changed pages are one write; a longer tail is new.
        let mut third = second.clone();
        third[0] = 1;
        third.extend_from_slice(&[7, 7, 7, 7]);
        assert_eq!(writes(&mut m, 0, &third), [(0, third[0..4].to_vec()), (16, vec![7; 4])]);
        assert_eq!(writes(&mut m, 8, &[9; 6]), [(8, vec![9; 6])]);
        let all = m.bytes.clone();
        assert!(writes(&mut m, 0, &all).is_empty());
    }

    #[test]
    fn bridged_writes_take_the_few_unchanged_pages_between_changes_along() {
        let mut m = Mirror::default();
        let first = vec![0u8; 40];
        m.sync(0, &first, 4, |_, _| {});
        // Pages 1, 3, 7 and 9 change: one unchanged page lies between the
        // first two and the last two, three between the pairs.
        let mut second = first.clone();
        for page in [1, 3, 7, 9] {
            second[page * 4] = 1;
        }
        let mut out = Vec::new();
        m.sync_bridged(0, &second, 4, 1, |at, b| out.push((at, b.len())));
        assert_eq!(out, [(4, 12), (28, 12)], "runs end with their last changed page");
        let mut third = second.clone();
        third[0] = 2;
        third[39] = 2;
        out.clear();
        m.sync_bridged(0, &third, 4, 8, |at, b| out.push((at, b.len())));
        assert_eq!(out, [(0, 40)], "a wide bridge joins them all");
        out.clear();
        m.sync_bridged(0, &third, 4, 8, |at, b| out.push((at, b.len())));
        assert!(out.is_empty(), "and what was bridged is known");
    }

    #[test]
    fn forgotten_and_far_mirrors_write_everything() {
        let mut m = Mirror::default();
        writes(&mut m, 0, &[1; 8]);
        m.forget();
        assert_eq!(writes(&mut m, 0, &[1; 8]), [(0, vec![1; 8])]);
        assert_eq!(writes(&mut m, 20, &[2; 4]), [(20, vec![2; 4])], "past a gap");
        assert_eq!(writes(&mut m, 20, &[2; 4]), [(20, vec![2; 4])], "still unknown");
    }
}
