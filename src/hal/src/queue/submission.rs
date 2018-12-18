//! A `Submission` is simply a collection of data bundled up and ready
//! to be submitted to a command queue.

use {pso, Backend};

use std::borrow::Borrow;


/// Submission information for a command queue.
pub struct Submission<'a, B: Backend + 'a, IC> {
    /// Command buffers to submit.
    pub cmd_buffers: IC,
    /// Semaphores to wait being signalled before submission.
    pub wait_semaphores: &'a [(&'a B::Semaphore, pso::PipelineStage)],
    /// Semaphores which get signalled after submission.
    pub signal_semaphores: &'a [&'a B::Semaphore],
}
