//! Command queues.
//!
//! Queues are the execution paths of the graphical processing units. These process
//! submitted commands buffers.
//!
//! There are different types of queues, which can only handle associated command buffers.
//! `CommandQueue<B, C>` has the capability defined by `C`: graphics, compute and transfer.

pub mod family;

use crate::{
    device::OutOfMemory,
    pso,
    window::{PresentError, PresentationSurface, Suboptimal},
    Backend,
};
use std::{
    any::Any,
    borrow::{Borrow, BorrowMut},
    fmt, iter,
};

pub use self::family::{QueueFamily, QueueFamilyId, QueueGroup};
use crate::memory::{SparseBind, SparseImageBind};

/// The type of the queue, an enum encompassing `queue::Capability`
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum QueueType {
    /// Supports all operations.
    General,
    /// Only supports graphics and transfer operations.
    Graphics,
    /// Only supports compute and transfer operations.
    Compute,
    /// Only supports transfer operations.
    Transfer,
}

impl QueueType {
    /// Returns true if the queue supports graphics operations.
    pub fn supports_graphics(&self) -> bool {
        match *self {
            QueueType::General | QueueType::Graphics => true,
            QueueType::Compute | QueueType::Transfer => false,
        }
    }
    /// Returns true if the queue supports compute operations.
    pub fn supports_compute(&self) -> bool {
        match *self {
            QueueType::General | QueueType::Graphics | QueueType::Compute => true,
            QueueType::Transfer => false,
        }
    }
    /// Returns true if the queue supports transfer operations.
    pub fn supports_transfer(&self) -> bool {
        true
    }
}

/// Scheduling hint for devices about the priority of a queue.  Values range from `0.0` (low) to
/// `1.0` (high).
pub type QueuePriority = f32;

/// Submission information for a [command queue][CommandQueue].
///
/// The submission is sent to the device through the [`submit`][CommandQueue::submit] method.
#[derive(Debug)]
pub struct Submission<Ic, Iw, Is> {
    /// Command buffers to submit.
    pub command_buffers: Ic,
    /// Semaphores to wait being signalled before submission.
    pub wait_semaphores: Iw,
    /// Semaphores to signal after all command buffers in the submission have finished execution.
    pub signal_semaphores: Is,
}

/// Information for binding sparse memory using a [command queue][CommandQueue].
///
/// The info is sent to the device through the [`bind_sparse`][CommandQueue::bind_sparse] method.
#[derive(Debug)]
pub struct BindSparseInfo<Iw, Is, Ib, Io, Ii> {
    /// Semaphores to wait being signalled before submission.
    pub wait_semaphores: Iw,
    /// Semaphores to signal after all command buffers in the submission have finished execution.
    pub signal_semaphores: Is,
    /// Defines sparse buffer memory bind operations.
    pub buffer_memory_binds: Ib,
    /// Defines sparse image opaque memory bind operations.
    pub image_opaque_memory_binds: Io,
    /// Defines sparse image memory bind operations.
    pub image_memory_binds: Ii,
}

/// Abstraction for an internal GPU execution engine.
///
/// Commands are executed on the the device by submitting
/// [command buffers][crate::command::CommandBuffer].
///
/// Queues can also be used for presenting to a surface
/// (that is, flip the front buffer with the next one in the chain).
pub trait CommandQueue<B: Backend>: fmt::Debug + Any + Send + Sync {
    /// Submit command buffers to queue for execution.
    ///
    /// # Arguments
    ///
    /// * `submission` - information about which command buffers to submit,
    ///   as well as what semaphores to wait for or to signal when done.
    /// * `fence` - must be in unsignaled state, and will be signaled after
    ///   all command buffers in the submission have finished execution.
    ///
    /// # Safety
    ///
    /// It's not checked that the queue can process the submitted command buffers.
    ///
    /// For example, trying to submit compute commands to a graphics queue
    /// will result in undefined behavior.
    unsafe fn submit<'a, T, Ic, S, Iw, Is>(
        &mut self,
        submission: Submission<Ic, Iw, Is>,
        fence: Option<&B::Fence>,
    ) where
        T: 'a + Borrow<B::CommandBuffer>,
        Ic: IntoIterator<Item = &'a T>,
        S: 'a + Borrow<B::Semaphore>,
        Iw: IntoIterator<Item = (&'a S, pso::PipelineStage)>,
        Is: IntoIterator<Item = &'a S>;

    /// Sparse memory bind operation.
    ///
    /// # Arguments
    ///
    /// * `info` - information about the memory bindings.
    ///
    /// # Safety
    ///
    /// - Defining memory as `None` will cause undefined behaviour when the
    /// tile is read or written from in some hardware.
    /// - The memory regions provided are not checked to be valid and matching
    /// of the sparse resource type.
    /// - If extents are not a multiple of the block size, additional space will be
    /// bound, and accessing memory is unsafe.
    unsafe fn bind_sparse<'a, M, Bf, I, S, Iw, Is, Ibi, Ib, Iii, Io, Ii>(
        &mut self,
        info: BindSparseInfo<Iw, Is, Ib, Io, Ii>,
        device: &B::Device,
        fence: Option<&B::Fence>,
    ) where
        Bf: 'a + BorrowMut<B::Buffer>,
        M: 'a + Borrow<B::Memory>,
        Ibi: IntoIterator<Item = SparseBind<&'a M>>,
        Ib: IntoIterator<Item = (&'a mut Bf, Ibi)>,
        I: 'a + BorrowMut<B::Image>,
        Iii: IntoIterator<Item = SparseImageBind<&'a M>>,
        Io: IntoIterator<Item = (&'a mut I, Ibi)>,
        Ii: IntoIterator<Item = (&'a mut I, Iii)>,
        S: 'a + Borrow<B::Semaphore>,
        Iw: IntoIterator<Item = &'a S>,
        Is: IntoIterator<Item = &'a S>,
        Ibi::IntoIter: ExactSizeIterator,
        Ib::IntoIter: ExactSizeIterator,
        Iii::IntoIter: ExactSizeIterator,
        Io::IntoIter: ExactSizeIterator,
        Ii::IntoIter: ExactSizeIterator;

    /// Simplified version of `submit` that doesn't expect any semaphores.
    unsafe fn submit_without_semaphores<'a, T, Ic>(
        &mut self,
        command_buffers: Ic,
        fence: Option<&B::Fence>,
    ) where
        T: 'a + Borrow<B::CommandBuffer>,
        Ic: IntoIterator<Item = &'a T>,
    {
        let submission = Submission {
            command_buffers,
            wait_semaphores: iter::empty(),
            signal_semaphores: iter::empty(),
        };
        self.submit::<_, _, B::Semaphore, _, _>(submission, fence)
    }

    /// Present a swapchain image directly to a surface, after waiting on `wait_semaphore`.
    ///
    /// # Safety
    ///
    /// Unsafe for the same reasons as [`submit`][CommandQueue::submit].
    /// No checks are performed to verify that this queue supports present operations.
    unsafe fn present(
        &mut self,
        surface: &mut B::Surface,
        image: <B::Surface as PresentationSurface<B>>::SwapchainImage,
        wait_semaphore: Option<&B::Semaphore>,
    ) -> Result<Option<Suboptimal>, PresentError>;

    /// Wait for the queue to be idle.
    fn wait_idle(&self) -> Result<(), OutOfMemory>;
}
