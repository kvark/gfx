//! Command buffers.
//!
//! A command buffer collects a list of commands to be submitted to the device.
//! Each command buffer has specific capabilities for graphics, compute or transfer operations,
//! and can be either a "primary" command buffer or a "secondary" command buffer.  Operations
//! always start from a primary command buffer, but a primary command buffer can contain calls
//! to secondary command buffers that contain snippets of commands that do specific things, similar
//! to function calls.
//!
//! All the possible commands are implemented in the `RawCommandBuffer` trait, and then the `CommandBuffer`
//! and related types make a generic, strongly-typed wrapper around it that only expose the methods that
//! are valid for the capabilities it provides.

// TODO: Document pipelines and subpasses better.

use Backend;
use queue::capability::Supports;

use std::borrow::Borrow;
use std::marker::PhantomData;


mod compute;
mod graphics;
mod raw;
mod render_pass;
mod transfer;

pub use self::graphics::*;
pub use self::raw::{
    ClearValueRaw, ClearColorRaw, ClearDepthStencilRaw, DescriptorSetOffset,
    RawCommandBuffer, CommandBufferFlags, Level as RawLevel, CommandBufferInheritanceInfo,
};
pub use self::render_pass::*;
pub use self::transfer::*;


/// Trait indicating how many times a Submit object can be submitted to a command buffer.
pub trait Shot {
    ///
    const FLAGS: CommandBufferFlags;
}
/// Indicates a Submit that can only be submitted once.
pub enum OneShot { }
impl Shot for OneShot { const FLAGS: CommandBufferFlags = CommandBufferFlags::ONE_TIME_SUBMIT; }

/// Indicates a Submit that can be submitted multiple times.
pub enum MultiShot { }
impl Shot for MultiShot { const FLAGS: CommandBufferFlags = CommandBufferFlags::EMPTY; }

/// A trait indicating the level of a command buffer.
pub trait Level { }

/// Indicates a primary command buffer.

/// Vulkan describes a primary command buffer as one which can be directly submitted
/// to a queue, and can execute `Secondary` command buffers.
pub enum Primary { }
impl Level for Primary { }

/// Indicates a secondary command buffer.
///
/// Vulkan describes a secondary command buffer as one which cannot be directly submitted
/// to a queue, but can be executed by a primary command buffer. This allows
/// multiple secondary command buffers to be constructed which do specific
/// things and can then be composed together into primary command buffers.
pub enum Secondary { }
impl Level for Secondary { }


/// A convenience alias for not typing out the full signature of a secondary command buffer.
pub type SecondaryCommandBuffer<B, C, S = OneShot> = CommandBuffer<B, C, S, Secondary>;

/// A strongly-typed command buffer that will only implement methods that are valid for the operations
/// it supports.
pub struct CommandBuffer<B: Backend, C, S = OneShot, L = Primary, R = <B as Backend>::CommandBuffer> {
    pub(crate) raw: R,
    pub(crate) _marker: PhantomData<(B, C, S, L)>
}

//TODO: avoid the `R` generic magic
impl<B: Backend, C, S, L, R> Borrow<R> for CommandBuffer<B, C, S, L, R>
where
    B: Backend<CommandBuffer = R>,
    R: RawCommandBuffer<B>,
{
    fn borrow(&self) -> &B::CommandBuffer {
        &self.raw
    }
}

impl<B: Backend, C, S: Shot> CommandBuffer<B, C, S, Primary> {
    /// Begin recording a primary command buffer.
    pub fn begin(
        &mut self,
        allow_pending_resubmit: bool,
    ) {
        let mut flags = S::FLAGS;
        if allow_pending_resubmit {
            flags |= CommandBufferFlags::SIMULTANEOUS_USE;
        }
        self.raw.begin(flags, CommandBufferInheritanceInfo::default());
    }
}

impl<B: Backend, C, S: Shot> CommandBuffer<B, C, S, Secondary> {
    /// Begin recording a secondary command buffer.
    pub fn begin(
        &mut self,
        allow_pending_resubmit: bool,
        inheritance: CommandBufferInheritanceInfo<B>,
    ) {
        let mut flags = S::FLAGS;
        if allow_pending_resubmit {
            flags |= CommandBufferFlags::SIMULTANEOUS_USE;
        }
        self.raw.begin(flags, inheritance);
    }
}

impl<B: Backend, C, S: Shot, L: Level> CommandBuffer<B, C, S, L> {
    /// Create a new typed command buffer from a raw command pool.
    pub unsafe fn new(raw: B::CommandBuffer) -> Self {
        CommandBuffer {
            raw,
            _marker: PhantomData,
        }
    }

    /// Finish recording commands to the command buffers.
    ///
    /// The command pool must be reset to able to re-record commands.
    pub fn finish(&mut self) {
        self.raw.finish();
    }

    /*
    /// Get a reference to the raw command buffer
    pub fn as_raw(&self) -> &B::CommandBuffer {
        &self.raw
    }

    /// Get a mutable reference to the raw command buffer
    pub fn as_raw_mut(&mut self) -> &mut B::CommandBuffer {
        &mut self.raw
    }*/

    /// Downgrade a command buffer to a lesser capability type.
    ///
    /// This is safe as a downgraded version can't be `submit`'ed
    /// since `submit` requires `self` by move.
    pub fn downgrade<D>(&mut self) -> &mut CommandBuffer<B, D, S>
    where
        C: Supports<D>
    {
        unsafe { ::std::mem::transmute(self) }
    }
}

impl<B: Backend, C, S: Shot> CommandBuffer<B, C, S, Primary> {
    /// Identical to the `RawCommandBuffer` method of the same name.
    pub fn execute_commands<I, K, P: Shot>(&mut self, cmd_buffers: I)
    where
        I: IntoIterator<Item = CommandBuffer<B, K, P, Secondary>>,
        C: Supports<K>,
    {
        self.raw.execute_commands(cmd_buffers);
    }
}
