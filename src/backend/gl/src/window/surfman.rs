//! Window creation using glutin for gfx.
//!
//! # Examples
//!
//! The following code creates a `gfx::Surface` using glutin.
//!
//! ```no_run
//! extern crate glutin;
//! extern crate gfx_backend_gl;
//!
//! fn main() {
//!     use gfx_backend_gl::Surface;
//!     use glutin::{ContextBuilder, WindowedContext};
//!     use glutin::window::WindowBuilder;
//!     use glutin::event_loop::EventLoop;
//!
//!     // First create a window using glutin.
//!     let mut events_loop = EventLoop::new();
//!     let wb = WindowBuilder::new();
//!     let glutin_window = ContextBuilder::new().with_vsync(true).build_windowed(wb, &events_loop).unwrap();
//!     let (glutin_context, glutin_window) = unsafe { glutin_window.make_current().expect("Failed to make the context current").split() };
//!
//!     // Then use the glutin window to create a gfx surface.
//!     let surface = Surface::from_context(glutin_context);
//! }
//! ```
//!
//! Headless initialization without a window.
//!
//! ```no_run
//! extern crate glutin;
//! extern crate gfx_backend_gl;
//! extern crate gfx_hal;
//!
//! use gfx_hal::Instance;
//! use gfx_backend_gl::Headless;
//! use glutin::{Context, ContextBuilder};
//! use glutin::event_loop::EventLoop;
//!
//! fn main() {
//!     let events_loop = EventLoop::new();
//!     let context = ContextBuilder::new().build_headless(&events_loop, glutin::dpi::PhysicalSize::new(0.0, 0.0))
//!         .expect("Failed to build headless context");
//!     let context = unsafe { context.make_current() }.expect("Failed to make the context current");
//!     let headless = Headless::from_context(context);
//!     let _adapters = headless.enumerate_adapters();
//! }
//! ```

use crate::{conv, native, Backend as B, Device, GlContainer, PhysicalDevice, QueueFamily, Starc};
use hal::{adapter::Adapter, format as f, image, window};

use arrayvec::ArrayVec;
use glow::HasContext;
use parking_lot::RwLock;
use surfman;

use std::iter;

#[derive(Debug)]
pub struct Swapchain {
    // Underlying window, required for presentation
    pub(crate) context: Starc<RwLock<surfman::Context>>,
    // Extent because the window lies
    pub(crate) extent: window::Extent2D,
    ///
    pub(crate) fbos: ArrayVec<[native::RawFrameBuffer; 3]>,
}

impl window::Swapchain<B> for Swapchain {
    unsafe fn acquire_image(
        &mut self,
        _timeout_ns: u64,
        _semaphore: Option<&native::Semaphore>,
        _fence: Option<&native::Fence>,
    ) -> Result<(window::SwapImageIndex, Option<window::Suboptimal>), window::AcquireError> {
        // TODO: sync
        Ok((0, None))
    }
}

#[derive(Debug)]
pub struct Instance {
    /// The surfman device. This is needed to create contexts, sufaces, etc. and make changes to
    /// them.
    pub(crate) device: Starc<RwLock<surfman::Device>>,
    /// The connection used by the surfman device
    pub(crate) connection: Starc<surfman::Connection>,
    /// Instance context. This context is not used for rendering, but is used when enumerating
    /// adapters.
    pub(crate) context: Starc<surfman::Context>,
}

impl Instance {
    pub unsafe fn create_surface_from_rwh(
        &self,
        raw_handle: raw_window_handle::RawWindowHandle,
    ) -> Surface {
        // Create a context for the surface
        let context_attributes = surfman::ContextAttributes {
            version: surfman::GLVersion::new(3, 3), // TODO: Figure out how to determine GL version
            flags: surfman::ContextAttributeFlags::empty(),
        };
        let context_descriptor = self
            .device
            .write()
            .create_context_descriptor(&context_attributes)
            .expect("TODO");
        let context = self
            .device
            .write()
            .create_context(&context_descriptor)
            .expect("TODO");

        // Create the surface with the context
        let surface = self
            .device
            .write()
            .create_surface(
                &context,
                surfman::SurfaceAccess::GPUOnly,
                surfman::SurfaceType::Widget {
                    // Create a native widget for the raw window handle
                    native_widget: self
                        .connection
                        .create_native_widget_from_rwh(raw_handle)
                        .expect("TODO"),
                },
            )
            .expect("TODO");

        // Create a surface with the given context
        Surface {
            renderbuffer: None,
            swapchain: None,
            context: Starc::new(RwLock::new(context)),
            surface: Starc::new(surface),
        }
    }
}

impl hal::Instance<B> for Instance {
    fn create(_: &str, _: u32) -> Result<Self, hal::UnsupportedBackend> {
        // TODO: I think this technically assumes that the default `surfman` device will match
        // the device type, i.e. x11 or wayland, of the raw window handle passed into
        // `create_surface`. This may be a fairly reasonable assumption, but that should be
        // verified.
        let connection = surfman::Connection::new().expect("TODO");
        let mut device = connection
            .create_device(&connection.create_hardware_adapter().expect("TODO"))
            .expect("TODO");

        let context_attributes = surfman::ContextAttributes {
            version: surfman::GLVersion::new(3, 3), // TODO: Figure out how to determine GL version
            flags: surfman::ContextAttributeFlags::empty(),
        };
        let context_descriptor = device
            .create_context_descriptor(&context_attributes)
            .expect("TODO");
        let context = device.create_context(&context_descriptor).expect("TODO");

        Ok(Instance {
            device: Starc::new(RwLock::new(device)),
            connection: Starc::new(connection),
            context: Starc::new(context),
        })
    }

    fn enumerate_adapters(&self) -> Vec<Adapter<B>> {
        // Create gl container
        let mut gl = GlContainer::from_fn_proc(|symbol_name| {
            self.device
                .write()
                .get_proc_address(&self.context, symbol_name) as *const _
        });

        // Create physical device
        let adapter = PhysicalDevice::new_adapter((), gl);
        vec![adapter]
    }

    unsafe fn create_surface(
        &self,
        has_handle: &impl raw_window_handle::HasRawWindowHandle,
    ) -> Result<Surface, window::InitError> {
        Ok(self.create_surface_from_rwh(has_handle.raw_window_handle()))
    }

    unsafe fn destroy_surface(&self, mut surface: Surface) {
        self.device.write().destroy_surface(
            &mut surface.context.write(),
            Starc::get_mut(&mut surface.surface).expect("TODO"),
        );
        self.device
            .write()
            .destroy_context(&mut surface.context.write());
    }
}

//TODO: if we make `Surface` a `WindowBuilder` instead of `RawContext`,
// we could spawn window + GL context when a swapchain is requested
// and actually respect the swapchain configuration provided by the user.
#[derive(Debug)]
pub struct Surface {
    pub(crate) surface: Starc<surfman::Surface>,
    pub(crate) context: Starc<RwLock<surfman::Context>>,
    pub(crate) swapchain: Option<Swapchain>,
    renderbuffer: Option<native::Renderbuffer>,
}

impl Surface {
    // pub fn from_context(context: surfman::Context) -> Self {
    //     Surface {
    //         renderbuffer: None,
    //         swapchain: None,
    //         context: Starc::new(context),
    //     }
    // }

    pub fn context(&self) -> Starc<RwLock<surfman::Context>> {
        self.context.clone()
    }

    fn swapchain_formats(&self) -> Vec<f::Format> {
        // let pixel_format = self.context.get_pixel_format();
        // let color_bits = pixel_format.color_bits;
        // let alpha_bits = pixel_format.alpha_bits;
        // let srgb = pixel_format.srgb;

        // TODO: expose more formats
        // match (color_bits, alpha_bits, srgb) {
        //     (24, 8, true) => vec![f::Format::Rgba8Srgb, f::Format::Bgra8Srgb],
        //     (24, 8, false) => vec![f::Format::Rgba8Unorm, f::Format::Bgra8Unorm],
        //     _ => vec![],
        // }
        // TODO: Figure out how to get pixel format from surfman
        vec![f::Format::Rgba8Srgb, f::Format::Bgra8Srgb]
    }
}

impl window::PresentationSurface<B> for Surface {
    type SwapchainImage = native::ImageView;

    unsafe fn configure_swapchain(
        &mut self,
        device: &Device,
        config: window::SwapchainConfig,
    ) -> Result<(), window::CreationError> {
        let gl = &device.share.context;

        if let Some(old) = self.swapchain.take() {
            for fbo in old.fbos {
                gl.delete_framebuffer(fbo);
            }
        }

        if self.renderbuffer.is_none() {
            self.renderbuffer = Some(gl.create_renderbuffer().unwrap());
        }

        let desc = conv::describe_format(config.format).unwrap();
        gl.bind_renderbuffer(glow::RENDERBUFFER, self.renderbuffer);
        gl.renderbuffer_storage(
            glow::RENDERBUFFER,
            desc.tex_internal,
            config.extent.width as i32,
            config.extent.height as i32,
        );

        let fbo = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(fbo));
        gl.framebuffer_renderbuffer(
            glow::READ_FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::RENDERBUFFER,
            self.renderbuffer,
        );
        self.swapchain = Some(Swapchain {
            context: self.context.clone(),
            extent: config.extent,
            fbos: iter::once(fbo).collect(),
        });

        Ok(())
    }

    unsafe fn unconfigure_swapchain(&mut self, device: &Device) {
        let gl = &device.share.context;
        if let Some(old) = self.swapchain.take() {
            for fbo in old.fbos {
                gl.delete_framebuffer(fbo);
            }
        }
        if let Some(rbo) = self.renderbuffer.take() {
            gl.delete_renderbuffer(rbo);
        }
    }

    unsafe fn acquire_image(
        &mut self,
        _timeout_ns: u64,
    ) -> Result<(Self::SwapchainImage, Option<window::Suboptimal>), window::AcquireError> {
        let image = native::ImageView::Renderbuffer(self.renderbuffer.unwrap());
        Ok((image, None))
    }
}

impl window::Surface<B> for Surface {
    fn supports_queue_family(&self, _: &QueueFamily) -> bool {
        true
    }

    fn capabilities(&self, _physical_device: &PhysicalDevice) -> window::SurfaceCapabilities {
        window::SurfaceCapabilities {
            present_modes: window::PresentMode::FIFO, //TODO
            composite_alpha_modes: window::CompositeAlphaMode::OPAQUE, //TODO
            // TODO: Figure out how to get pixel format from surfman
            // image_count: if self.context.get_pixel_format().double_buffer {
            //     2..=2
            // } else {
            //     1..=1
            // },
            image_count: 1 ..= 1,
            current_extent: None,
            extents: window::Extent2D {
                width: 4,
                height: 4,
            } ..= window::Extent2D {
                width: 4096,
                height: 4096,
            },
            max_image_layers: 1,
            usage: image::Usage::COLOR_ATTACHMENT | image::Usage::TRANSFER_SRC,
        }
    }

    fn supported_formats(&self, _physical_device: &PhysicalDevice) -> Option<Vec<f::Format>> {
        Some(self.swapchain_formats())
    }
}

// pub fn config_context<C>(
//     builder: glutin::ContextBuilder<C>,
//     color_format: f::Format,
//     ds_format: Option<f::Format>,
// ) -> glutin::ContextBuilder<C>
// where
//     C: glutin::ContextCurrentState,
// {
//     let color_base = color_format.base_format();
//     let color_bits = color_base.0.describe_bits();
//     let depth_bits = match ds_format {
//         Some(fm) => fm.base_format().0.describe_bits(),
//         None => f::BITS_ZERO,
//     };
//     builder
//         .with_depth_buffer(depth_bits.depth)
//         .with_stencil_buffer(depth_bits.stencil)
//         .with_pixel_format(color_bits.color, color_bits.alpha)
//         .with_srgb(color_base.1 == f::ChannelType::Srgb)
// }

// #[derive(Debug)]
// pub struct Headless {
//     pub context: Starc<surfman::Context>,
//     pub device: Starc<surfman::Device>,
// }

// impl hal::Instance<B> for Headless {
//     fn create(_: &str, _: u32) -> Result<Self, hal::UnsupportedBackend> {
//         use surfman::{
//             Adapter, Connection, ContextAttributeFlags, ContextAttributes, Device, GLVersion,
//         };
//         let connection = Connection::new().expect("TODO");
//         let device = Device::new(&connection, &Adapter::hardware().expect("TODO"));
//         let context_attributes = ContextAttributes {
//             version: GLVersion::new(3, 3),
//             flags: ContextAttributeFlags::empty(),
//         };
//         let context_descriptor = device
//             .create_context_descriptor(&context_attributes)
//             .expect("TODO");
//         let context = device.create_context(&context_descriptor).expect("TODO");

//         Ok(Headless {
//             device: Starc::new(device),
//             context: Starc::new(context),
//         })
//     }

//     fn enumerate_adapters(&self) -> Vec<Adapter<B>> {
//         let adapter = PhysicalDevice::new_adapter(
//             (),
//             GlContainer::from_fn_proc(|s| self.0.get_proc_address(s) as *const _),
//         );
//         vec![adapter]
//     }

//     unsafe fn create_surface(
//         &self,
//         _: &impl raw_window_handle::HasRawWindowHandle,
//     ) -> Result<Surface, window::InitError> {
//         use surfman::{SurfaceAccess, SurfaceType};
//         let surface = self
//             .device
//             .create_surface(
//                 &self.context,
//                 SurfaceAccess::GPUOnly,
//                 &SurfaceType::Generic {
//                     // TODO: Make headless surface size configurable
//                     size: euclid::default::Size2D::new(640, 480),
//                 },
//             )
//             .expect("TODO");

//         Ok(surface)
//     }

//     unsafe fn destroy_surface(&self, surface: Surface) {
//         self.device.destroy_surface(&self.context, surface);
//     }
// }
