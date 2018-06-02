extern crate gfx_hal as hal;
#[macro_use] extern crate bitflags;
extern crate cocoa;
#[macro_use]
extern crate derivative;
extern crate foreign_types;
#[macro_use] extern crate objc;
extern crate io_surface;
extern crate core_foundation;
extern crate core_graphics;
#[macro_use] extern crate log;
extern crate block;
extern crate smallvec;
extern crate spirv_cross;

extern crate metal_rs as metal;

#[cfg(feature = "winit")]
extern crate winit;

#[path = "../../auxil/range_alloc.rs"]
mod range_alloc;
mod device;
mod window;
mod command;
mod internal;
mod native;
mod conversions;
mod soft;

pub use command::CommandPool;
pub use device::{Device, LanguageVersion, PhysicalDevice};
pub use window::{Surface, Swapchain};

pub type GraphicsCommandPool = CommandPool;

use std::mem;
use std::sync::{Arc, Mutex};
use std::os::raw::c_void;

use hal::queue::QueueFamilyId;

use objc::runtime::{Object, Class};
use cocoa::base::YES;
use cocoa::foundation::NSAutoreleasePool;
use core_graphics::geometry::CGRect;


const MAX_ACTIVE_COMMAND_BUFFERS: usize = 1 << 14;

#[derive(Debug, Clone, Copy)]
pub struct QueueFamily {}

impl hal::QueueFamily for QueueFamily {
    fn queue_type(&self) -> hal::QueueType { hal::QueueType::General }
    fn max_queues(&self) -> usize { 1 }
    fn id(&self) -> QueueFamilyId { QueueFamilyId(0) }
}

struct Shared {
    device: Mutex<metal::Device>,
    queue: Mutex<command::QueueInner>,
    service_pipes: Mutex<internal::ServicePipes>,
    push_constants_buffer_id: u32,
    disabilities: PrivateDisabilities,
}

unsafe impl Send for Shared {}
unsafe impl Sync for Shared {}

impl Shared {
    fn new(device: metal::Device) -> Self {
        let feature_macos_10_14: metal::MTLFeatureSet = unsafe { mem::transmute(10004u64) };
        Shared {
            queue: Mutex::new(command::QueueInner::new(&device, MAX_ACTIVE_COMMAND_BUFFERS)),
            service_pipes: Mutex::new(internal::ServicePipes::new(&device)),
            push_constants_buffer_id: 30,
            disabilities: PrivateDisabilities {
                broken_viewport_near_depth: device.name().starts_with("Intel") &&
                    !device.supports_feature_set(feature_macos_10_14),
            },
            device: Mutex::new(device),
        }
    }
}


pub struct Instance {
    shared: Arc<Shared>,
}

impl hal::Instance for Instance {
    type Backend = Backend;

    fn enumerate_adapters(&self) -> Vec<hal::Adapter<Backend>> {
        // TODO: enumerate all devices
        let name = self.shared.device.lock().unwrap().name().into();

        vec![
            hal::Adapter {
                info: hal::AdapterInfo {
                    name,
                    vendor: 0,
                    device: 0,
                    software_rendering: false,
                },
                physical_device: device::PhysicalDevice::new(self.shared.clone()),
                queue_families: vec![QueueFamily{}],
            }
        ]
    }
}

impl Instance {
    pub fn create(_: &str, _: u32) -> Self {
        let device = metal::Device::system_default();
        Instance {
            shared: Arc::new(Shared::new(device)),
        }
    }

    pub fn create_surface_from_nsview(&self, nsview: *mut c_void) -> Surface {
        unsafe {
            let view: cocoa::base::id = mem::transmute(nsview);
            if view.is_null() {
                panic!("window does not have a valid contentView");
            }

            msg_send![view, setWantsLayer: YES];
            let render_layer: *mut Object = msg_send![Class::get("CALayer").unwrap(), new]; // Returns retained
            let view_size: CGRect = msg_send![view, bounds];
            msg_send![render_layer, setFrame: view_size];
            let view_layer: *mut Object = msg_send![view, layer];
            msg_send![view_layer, addSublayer: render_layer];

            msg_send![view, retain];
            window::Surface(Arc::new(window::SurfaceInner {
                nsview: view,
                render_layer: Mutex::new(render_layer),
            }))
        }
    }

    #[cfg(feature = "winit")]
    pub fn create_surface(&self, window: &winit::Window) -> Surface {
        use winit::os::macos::WindowExt;
        self.create_surface_from_nsview(window.get_nsview())
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum Backend {}
impl hal::Backend for Backend {
    type PhysicalDevice = device::PhysicalDevice;
    type Device = device::Device;

    type Surface = window::Surface;
    type Swapchain = window::Swapchain;

    type QueueFamily = QueueFamily;
    type CommandQueue = command::CommandQueue;
    type CommandBuffer = command::CommandBuffer;

    type Memory = native::Memory;
    type CommandPool = command::CommandPool;

    type ShaderModule = native::ShaderModule;
    type RenderPass = native::RenderPass;
    type Framebuffer = native::FrameBuffer;

    type UnboundBuffer = native::UnboundBuffer;
    type Buffer = native::Buffer;
    type BufferView = native::BufferView;
    type UnboundImage = native::UnboundImage;
    type Image = native::Image;
    type ImageView = native::ImageView;
    type Sampler = native::Sampler;

    type ComputePipeline = native::ComputePipeline;
    type GraphicsPipeline = native::GraphicsPipeline;
    type PipelineLayout = native::PipelineLayout;
    type DescriptorSetLayout = native::DescriptorSetLayout;
    type DescriptorPool = native::DescriptorPool;
    type DescriptorSet = native::DescriptorSet;

    type Fence = native::Fence;
    type Semaphore = native::Semaphore;
    type QueryPool = ();
}

#[derive(Clone, Copy, Debug)]
struct PrivateCapabilities {
    resource_heaps: bool,
    argument_buffers: bool,
    shared_textures: bool,
    format_depth24_stencil8: bool,
    format_depth32_stencil8: bool,
    format_min_srgb_channels: u8,
    format_b5: bool,
    max_buffers_per_stage: usize,
    max_textures_per_stage: usize,
    max_samplers_per_stage: usize,
    buffer_alignment: u64,
    max_buffer_size: u64,
}

#[derive(Clone, Copy, Debug)]
struct PrivateDisabilities {
    broken_viewport_near_depth: bool,
}

pub struct AutoreleasePool {
    pool: cocoa::base::id,
}

impl Drop for AutoreleasePool {
    fn drop(&mut self) {
        unsafe {
            msg_send![self.pool, release]
        }
    }
}

impl AutoreleasePool {
    pub fn new() -> Self {
        AutoreleasePool {
            pool: unsafe {
                NSAutoreleasePool::new(cocoa::base::nil)
            },
        }
    }

    pub unsafe fn reset(&mut self) {
        self.pool.drain();
        self.pool = NSAutoreleasePool::new(cocoa::base::nil);
    }
}

fn validate_line_width(width: f32) {
    // Note from the Vulkan spec:
    // > If the wide lines feature is not enabled, lineWidth must be 1.0
    // Simply assert and no-op because Metal never exposes `Features::LINE_WIDTH` 
    assert_eq!(width, 1.0);
}
