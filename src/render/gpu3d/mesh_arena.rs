//! Shared storage for immutable mesh geometry. Indices are rebased on upload,
//! so every tier (including WebGL2 without base-vertex draws) binds the whole
//! arena and draws with base vertex zero. Skinning keeps dedicated rest data.
//!
//! CPU mirrors permit growth before a queue is available at asset creation.
//! Only changed ranges upload at prepare; growth uploads the mirror once.
use super::resources::Vertex;
use crate::render::traffic::{Counted, NewBuffer, Upload};
use std::ops::Range;

#[derive(Clone)]
pub(super) struct Allocation {
    pub vertices: Range<u32>,
    pub indices: Range<u32>,
}

#[derive(Default)]
struct Arena<T> {
    data: Vec<T>,
    free: Vec<Range<u32>>,
    dirty: Vec<Range<u32>>,
    buffer: Option<wgpu::Buffer>,
}

impl<T: bytemuck::Pod + bytemuck::Zeroable> Arena<T> {
    fn allocate(&mut self, count: u32, limit: u64) -> Option<Range<u32>> {
        if let Some(i) = self.free.iter().position(|r| r.end - r.start >= count) {
            let start = self.free[i].start;
            self.free[i].start += count;
            if self.free[i].is_empty() {
                self.free.remove(i);
            }
            return Some(start..start + count);
        }
        let start = u32::try_from(self.data.len()).ok()?;
        let end = start.checked_add(count)?;
        if u64::from(end) * size_of::<T>() as u64 > limit {
            return None;
        }
        self.data.resize(end as usize, T::zeroed());
        Some(start..end)
    }

    fn release(&mut self, range: Range<u32>) {
        self.free.push(range);
        self.free.sort_unstable_by_key(|r| r.start);
        let mut i = 1;
        while i < self.free.len() {
            if self.free[i - 1].end == self.free[i].start {
                self.free[i - 1].end = self.free[i].end;
                self.free.remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn write(&mut self, range: Range<u32>, data: &[T]) {
        self.data[range.start as usize..range.end as usize].copy_from_slice(data);
        self.dirty.push(range);
    }

    fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, usage: wgpu::BufferUsages) {
        if self.dirty.is_empty() {
            return;
        }
        let bytes = bytemuck::cast_slice(&self.data);
        if self.buffer.as_ref().is_none_or(|b| b.size() < bytes.len() as u64) {
            // Geometric growth capped by the device limit. Four-byte alignment
            // holds for both Vertex and u32, including queue writes.
            let size = (bytes.len() as u64)
                .next_power_of_two()
                .min(device.limits().max_buffer_size);
            let buffer = device.new_buffer(&wgpu::BufferDescriptor {
                label: Some("shared mesh arena"),
                size,
                usage: usage
                    | wgpu::BufferUsages::COPY_DST
                    | if cfg!(test) {
                        wgpu::BufferUsages::COPY_SRC
                    } else {
                        wgpu::BufferUsages::empty()
                    },
                mapped_at_creation: false,
            });
            queue.uploading(Upload::Meshes).write_buffer(&buffer, 0, bytes);
            self.buffer = Some(buffer);
        } else {
            let buffer = self.buffer.as_ref().unwrap();
            // Coalesce adjacent uploads during asset bursts.
            self.dirty.sort_unstable_by_key(|r| r.start);
            let mut ranges = self.dirty.drain(..);
            let mut range = ranges.next().unwrap();
            for next in ranges {
                if next.start <= range.end {
                    range.end = range.end.max(next.end);
                } else {
                    queue.uploading(Upload::Meshes).write_buffer(
                        buffer,
                        range.start as u64 * size_of::<T>() as u64,
                        bytemuck::cast_slice(&self.data[range.start as usize..range.end as usize]),
                    );
                    range = next;
                }
            }
            queue.uploading(Upload::Meshes).write_buffer(
                buffer,
                range.start as u64 * size_of::<T>() as u64,
                bytemuck::cast_slice(&self.data[range.start as usize..range.end as usize]),
            );
        }
        self.dirty.clear();
    }
}

impl Default for Vertex {
    fn default() -> Self {
        bytemuck::Zeroable::zeroed()
    }
}

#[derive(Default)]
pub(super) struct MeshArena {
    vertices: Arena<Vertex>,
    indices: Arena<u32>,
    live: usize,
}

impl MeshArena {
    pub fn upload(&mut self, device: &wgpu::Device, vertices: &[Vertex], indices: &[u32]) -> Option<Allocation> {
        if vertices.is_empty() || indices.is_empty() {
            return None;
        }
        let limit = device.limits().max_buffer_size;
        let vertex_count = u32::try_from(vertices.len()).ok()?;
        let index_count = u32::try_from(indices.len()).ok()?;
        let v = self.vertices.allocate(vertex_count, limit)?;
        let Some(i) = self.indices.allocate(index_count, limit) else {
            self.vertices.release(v);
            return None;
        };
        self.vertices.write(v.clone(), vertices);
        // Keep this staging allocation at upload time, never per frame.
        let rebased: Vec<u32> = indices.iter().map(|index| index + v.start).collect();
        self.indices.write(i.clone(), &rebased);
        self.live += 1;
        Some(Allocation {
            vertices: v,
            indices: i,
        })
    }
    pub fn release(&mut self, allocation: Allocation) {
        self.vertices.release(allocation.vertices);
        self.indices.release(allocation.indices);
        self.live -= 1;
        if self.live == 0 {
            *self = Self::default();
        }
    }
    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.vertices.flush(device, queue, wgpu::BufferUsages::VERTEX);
        self.indices.flush(device, queue, wgpu::BufferUsages::INDEX);
    }
    pub fn buffers(&self) -> Option<(&wgpu::Buffer, &wgpu::Buffer)> {
        Some((self.vertices.buffer.as_ref()?, self.indices.buffer.as_ref()?))
    }
    pub fn bytes(&self) -> u64 {
        self.vertices.buffer.as_ref().map_or(0, wgpu::Buffer::size)
            + self.indices.buffer.as_ref().map_or(0, wgpu::Buffer::size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn freed_ranges_coalesce_and_limits_fall_back_without_overflow() {
        let mut arena = Arena::<u32>::default();
        let a = arena.allocate(3, 32).unwrap();
        let b = arena.allocate(5, 32).unwrap();
        assert!(arena.allocate(1, 32).is_none());
        arena.release(a);
        arena.release(b);
        assert_eq!(arena.allocate(8, 32), Some(0..8));
        assert!(arena.allocate(u32::MAX, 32).is_none());
    }
    #[test]
    fn gpu_arena_upload_reuse_and_growth_preserve_live_geometry() {
        let Some(gpu) = crate::render::render_test_support::gpu() else {
            return;
        };
        let (device, queue) = (&gpu.device, &gpu.queue);
        let mut arena = MeshArena::default();
        let vertices = vec![
            Vertex {
                position: [1.0, 2.0, 3.0],
                ..Default::default()
            };
            3
        ];
        let a = arena.upload(device, &vertices, &[0, 1, 2]).unwrap();
        let b = arena.upload(device, &vertices, &[2, 1, 0]).unwrap();
        arena.flush(device, queue);
        let check = |arena: &MeshArena, allocation: &Allocation, indices: &[u32]| {
            let (v, i) = arena.buffers().unwrap();
            let got = crate::render::traffic::read_back(
                device,
                queue,
                v,
                allocation.vertices.start as u64 * size_of::<Vertex>() as u64,
                vertices.len() as u64 * size_of::<Vertex>() as u64,
            );
            assert_eq!(got, bytemuck::cast_slice::<Vertex, u8>(&vertices));
            let got = crate::render::traffic::read_back(
                device,
                queue,
                i,
                allocation.indices.start as u64 * 4,
                indices.len() as u64 * 4,
            );
            let expected: Vec<u32> = indices.iter().map(|i| i + allocation.vertices.start).collect();
            assert_eq!(got, bytemuck::cast_slice::<u32, u8>(&expected));
        };
        check(&arena, &a, &[0, 1, 2]);
        check(&arena, &b, &[2, 1, 0]);
        arena.release(a.clone());
        let reused = arena.upload(device, &vertices, &[1, 2, 0]).unwrap();
        assert_eq!(reused.vertices, a.vertices);
        assert_eq!(reused.indices, a.indices);
        arena.flush(device, queue);
        check(&arena, &reused, &[1, 2, 0]);
        let big = arena
            .upload(device, &vec![Vertex::default(); 4096], &vec![0; 8192])
            .unwrap();
        arena.flush(device, queue);
        check(&arena, &b, &[2, 1, 0]);
        check(&arena, &reused, &[1, 2, 0]);
        arena.release(big);
        arena.release(b);
        arena.release(reused);
        assert_eq!(arena.bytes(), 0);
        assert!(arena.buffers().is_none());
        let again = arena.upload(device, &vertices, &[0, 2, 1]).unwrap();
        arena.flush(device, queue);
        check(&arena, &again, &[0, 2, 1]);
    }
}
