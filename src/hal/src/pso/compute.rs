//! Compute pipeline descriptor.

use Backend;
use super::{BasePipeline, EntryPoint, PipelineCreationFlags};

/// A description of the data needed to construct a compute pipeline.
#[derive(Debug)]
pub struct ComputePipelineDesc<'a, B: Backend, V: 'a> {
    /// The shader entry point that performs the computation.
    pub shader: EntryPoint<'a, B, V>,
    /// Pipeline layout.
    pub layout: &'a B::PipelineLayout,
    /// Any flags necessary for the pipeline creation.
    pub flags: PipelineCreationFlags,
    /// The parent pipeline to this one, if any.
    pub parent: BasePipeline<'a, B::ComputePipeline>,
}

impl<'a, B: Backend, V> ComputePipelineDesc<'a, B, V> {
    /// Create a new empty PSO descriptor.
    pub fn new(
        shader: EntryPoint<'a, B, V>,
        layout: &'a B::PipelineLayout,
    ) -> Self {
        ComputePipelineDesc {
            shader,
            layout,
            flags: PipelineCreationFlags::empty(),
            parent: BasePipeline::None,
        }
    }
}
