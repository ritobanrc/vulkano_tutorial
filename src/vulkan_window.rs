use std::sync::Arc;
use vulkano::instance::Instance;
use vulkano::framebuffer::Framebuffer;
use vulkano::framebuffer::FramebufferAbstract;
use vulkano::framebuffer::RenderPassAbstract;
use vulkano::swapchain::{SurfaceTransform, Swapchain, PresentMode};
use vulkano::swapchain::SwapchainCreationError;
use vulkano::swapchain::Surface;
use vulkano_win::VkSurfaceBuild;
use winit::EventsLoop;
use winit::WindowBuilder;
use winit::Window;
use winit::dpi::LogicalSize;

use crate::vulkan_init::VulkanInit;

pub struct VulkanWindow {
    pub events_loop: EventsLoop,
    pub dimensions: [u32; 2],
    pub surface: Arc<Surface<Window>>,
    pub swapchain: Arc<Swapchain<Window>>,
    pub render_pass: Arc<RenderPassAbstract + Send + Sync>,
    pub framebuffers: Vec<Arc<FramebufferAbstract + Send + Sync>>
        //window: &'a Window,
}

impl VulkanWindow {
    pub fn create(initializer: &VulkanInit, instance: Arc<Instance>) -> VulkanWindow {
        // Step 6: Create windows with event loop
        let events_loop = EventsLoop::new();
        let surface = WindowBuilder::new()
            .with_title("Vulkano Experiments")
            .with_dimensions(LogicalSize::new(600.0, 600.0))
            //.with_resizable(false)
            .build_vk_surface(&events_loop, instance.clone()).unwrap();
        //let window = surface.window();


        let (swapchain, images) =        {
            // Step 7: get the capabilities of the surface
            let caps = surface.capabilities(initializer.physical).unwrap();

            let dimensions = if let Some(dimensions) = surface.window().get_inner_size() {
                // convert to physical pixels
                let dimensions: (u32, u32) = dimensions.to_physical(surface.window().get_hidpi_factor()).into();
                [dimensions.0, dimensions.1]
            } else {
                // The window no longer exists so exit the application.
                panic!("Unable to get dimension");
            };

            let alpha = caps.supported_composite_alpha.iter().next().unwrap();
            let format = caps.supported_formats[0].0;

            // Step 8: Create a swapchain
            Swapchain::new(initializer.device.clone(), surface.clone(),
            caps.min_image_count, format, dimensions, 1, caps.supported_usage_flags, &initializer.queue, 
            SurfaceTransform::Identity, alpha, PresentMode::Fifo, true, None).unwrap()
        };

        // Step 10: Setup render pass
        let render_pass = Arc::new(vulkano::single_pass_renderpass!(initializer.device.clone(), 
                                                                    attachments: {
                                                                        color: {
                                                                            load: Clear,
                                                                            store: Store,
                                                                            format: swapchain.format(),
                                                                            samples: 1,
                                                                        }
                                                                    },
                                                                    pass: {
                                                                        color: [color],
                                                                        depth_stencil: {}
                                                                    }).unwrap());



        // Step 13: Create Frame buffers from dynamic state, render passes, and swapchain images
        let framebuffers = images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                .add(image.clone()).unwrap()
                .build().unwrap()) as Arc<FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>();


        VulkanWindow {
            events_loop: events_loop,
            dimensions: images[0].dimensions(),
            surface: surface,
            swapchain: swapchain,
            render_pass: render_pass,
            framebuffers: framebuffers,
            //window: window
        }
    }

    pub fn recreate_swapchain(&mut self) {
        let dimensions = if let Some(dimensions) = self.window().get_inner_size() {
            // convert to physical pixels
            let dimensions: (u32, u32) = dimensions.to_physical(self.window().get_hidpi_factor()).into();
            [dimensions.0, dimensions.1]
        } else {
            // The window no longer exists so exit the application.
            return;
        };


        // Step 8: Create a swapchain
        let (new_swapchain, new_images) = match self.swapchain.recreate_with_dimension(dimensions) {
            Ok(r) => r, 
            // The user is in the process of resizing or smth. Just keep going. What could possibly go wrong?!
            Err(SwapchainCreationError::UnsupportedDimensions) => return,
            Err(err) => panic!("{:?}", err),
        };

        self.swapchain = new_swapchain;


        // Step 13: Create Frame buffers from dynamic state, render passes, and swapchain images
        self.framebuffers = new_images.iter().map(|image| {
            Arc::new(
                Framebuffer::start(self.render_pass.clone())
                .add(image.clone()).unwrap()
                .build().unwrap()) as Arc<FramebufferAbstract + Send + Sync>
        }).collect::<Vec<_>>();

        self.dimensions = new_images[0].dimensions();
    }

    #[inline(always)]
    pub fn window(&self) -> &Window {
        self.surface.window()
    }
}

