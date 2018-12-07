//! Command pools

use {pass};
use {Backend};
use command::{
    CommandBuffer, RawCommandBuffer, SecondaryCommandBuffer,
    SubpassCommandBuffer, CommandBufferFlags, Shot, RawLevel,
    CommandBufferInheritanceInfo
};
use queue::capability::{Supports, Graphics};

use std::any::Any;
use std::marker::PhantomData;

bitflags!(
    /// Command pool creation flags.
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct CommandPoolCreateFlags: u8 {
        /// Indicates short-lived command buffers.
        /// Memory optimization hint for implementations.
        const TRANSIENT = 0x1;
        /// Allow command buffers to be reset individually.
        const RESET_INDIVIDUAL = 0x2;
    }
);

/// The allocated command buffers are associated with the creating command queue.
pub trait RawCommandPool<B: Backend>: Any + Send + Sync {
    /// Reset the command pool and the corresponding command buffers.
    ///
    /// # Synchronization: You may _not_ free the pool if a command buffer is still in use (pool memory still in use)
    fn reset(&mut self);

    /// Allocate a single command buffers from the pool.
    fn allocate_one(&mut self, level: RawLevel) -> B::CommandBuffer {
        self.allocate_vec(1, level).pop().unwrap()
    }

    /// Allocate new command buffers from the pool.
    fn allocate_vec(&mut self, num: usize, level: RawLevel) -> Vec<B::CommandBuffer> {
        (0 .. num).map(|_| self.allocate_one(level)).collect()
    }

    /// Free command buffers which are allocated from this pool.
    unsafe fn free<I>(&mut self, buffers: I)
    where I: IntoIterator<Item = B::CommandBuffer>;
}

/// Strong-typed command pool.
///
/// This a safer wrapper around `RawCommandPool` which ensures that only **one**
/// command buffer is recorded at the same time from the current queue.
/// Command buffers are stored internally and can only be obtained via a strong-typed
/// `CommandBuffer` wrapper for encoding.
pub struct CommandPool<B: Backend, C> {
    raw: B::CommandPool,
    _capability: PhantomData<C>,
}

impl<B: Backend, C> CommandPool<B, C> {
    /// Create typed command pool from raw.
    ///
    /// # Safety
    ///
    /// `<C as Capability>::supported_by(queue_type)` must return true
    /// for `queue_type` being the type of queues from family this `raw` pool is associated with.
    ///
    pub unsafe fn new(raw: B::CommandPool) -> Self {
        CommandPool {
            raw,
            _capability: PhantomData,
        }
    }

    /// Reset the command pool and the corresponding command buffers.
    ///
    /// # Synchronization: You may _not_ free the pool if a command buffer is still in use (pool memory still in use)
    pub fn reset(&mut self) {
        self.raw.reset();
    }

    /// Get a primary command buffer for recording.
    ///
    /// You can only record to one command buffer per pool at the same time.
    /// If more command buffers are requested than allocated, new buffers will be reserved.
    /// The command buffer will be returned in 'recording' state.
    pub fn acquire_command_buffer<S: Shot>(&mut self) -> CommandBuffer<B, C, S> {
        let buffer = self.raw.allocate_one(RawLevel::Primary);
        unsafe {
            CommandBuffer::new(buffer)
        }
    }

    /// Get a secondary command buffer for recording.
    ///
    /// You can only record to one command buffer per pool at the same time.
    /// If more command buffers are requested than allocated, new buffers will be reserved.
    /// The command buffer will be returned in 'recording' state.
    pub fn acquire_secondary_command_buffer<S: Shot>(&mut self) -> SecondaryCommandBuffer<B, C, S> {
        let buffer = self.raw.allocate_one(RawLevel::Secondary);
        unsafe {
            SecondaryCommandBuffer::new(buffer)
        }
    }

    /// Downgrade a typed command pool to untyped one, free up the allocated command buffers.
    pub fn into_raw(self) -> B::CommandPool {
        self.raw
    }
}

impl<B: Backend, C: Supports<Graphics>> CommandPool<B, C> {
    /// Get a subpass command buffer for recording.
    ///
    /// You can only record to one command buffer per pool at the same time.
    /// If more command buffers are requested than allocated, new buffers will be reserved.
    /// The command buffer will be returned in 'recording' state.
    pub fn acquire_subpass_command_buffer<'a, S: Shot>(
        &mut self,
        allow_pending_resubmit: bool,
        subpass: pass::Subpass<'a, B>,
        framebuffer: Option<&'a B::Framebuffer>,
    ) -> SubpassCommandBuffer<B, S> {
        let mut buffer = self.raw.allocate_one(RawLevel::Secondary);
        let mut flags = S::FLAGS;
        if allow_pending_resubmit {
            flags |= CommandBufferFlags::SIMULTANEOUS_USE;
        }
        let inheritance_info = CommandBufferInheritanceInfo {
            subpass: Some(subpass),
            framebuffer,
            ..CommandBufferInheritanceInfo::default()
        };
        buffer.begin(flags, inheritance_info);
        unsafe {
            SubpassCommandBuffer::new(buffer)
        }
    }
}
