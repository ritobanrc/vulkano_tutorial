use std::sync::Arc;
use std::iter;
use vulkano::instance::Instance;
use vulkano::instance::PhysicalDevice;
use vulkano::device::Device;
use vulkano::device::Queue;
use vulkano::device::DeviceExtensions;
use vulkano::device::Features;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::framebuffer::Subpass;
use vulkano::framebuffer::Framebuffer;
use vulkano::framebuffer::FramebufferAbstract;
use vulkano::framebuffer::RenderPassAbstract;
use vulkano::swapchain::{SurfaceTransform, Swapchain, PresentMode};
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::buffer::BufferUsage;
use vulkano::swapchain;
use vulkano::swapchain::{SwapchainCreationError, AcquireError};
use vulkano::swapchain::Surface;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::DynamicState;
use vulkano::pipeline::viewport::Viewport;
use vulkano::sync;
use vulkano::sync::{GpuFuture, FlushError};
use vulkano_win::VkSurfaceBuild;
use winit::Event;
use winit::EventsLoop;
use winit::WindowBuilder;
use winit::WindowEvent;
use winit::Window;
use winit::dpi::LogicalSize;

struct Vertex {
    position: [f32; 2], 
}

struct VulkanInit<'a> {
    //instance: Arc<Instance>,
    physical: PhysicalDevice<'a>,
    device: Arc<Device>,
    queue: Arc<Queue>
}

impl<'a> VulkanInit<'a> {
    fn create(instance:&'a Arc<Instance>) -> VulkanInit<'a> {
        // Step 2: Find physical device
        // The iterator has the same lifetime as the instance.
        let physical = PhysicalDevice::enumerate(instance).next().expect("No physical devices available.");


        // Step 3: Find queue families
        for family in physical.queue_families() {
            println!("Found a queue family with {:?} queues", family.queues_count());
        }

        let queue_family = physical.queue_families()
            .find(|&q| q.supports_graphics())
            .expect("Couldn't find a graphical queue family.");

        // Step 4: Create Device and queue. 
        let (device, mut queues) = {
            let device_ext = DeviceExtensions  {
                khr_swapchain: true,
                .. DeviceExtensions::none()
            };
            Device::new(physical, &Features::none(), &device_ext, 
                        [(queue_family, 0.5)].iter().cloned())
                .expect("Failed to create device.")
        };

        let queue = queues.next().unwrap();

        VulkanInit { 
            physical: physical,
            device: device,
            queue: queue
        }
    }
}

fn create_instance() -> Arc<Instance> {
    // TODO: generalize this to not rendering to a window
    let extentions = vulkano_win::required_extensions();
    Instance::new(None, &extentions, None).expect("Failed to create instance.")
}

struct VulkanWindow {
    events_loop: EventsLoop,
    dimensions: [u32; 2],
    surface: Arc<Surface<Window>>,
    swapchain: Arc<Swapchain<Window>>,
    //images: Vec<Arc<SwapchainImage<Window>>>,
    render_pass: Arc<RenderPassAbstract + Send + Sync>,
    framebuffers: Vec<Arc<FramebufferAbstract + Send + Sync>>
    //window: &'a Window,
}

impl VulkanWindow {
    fn create(initializer: &VulkanInit, instance: Arc<Instance>) -> VulkanWindow {
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

    fn recreate_swapchain(&mut self) {
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
    fn window(&self) -> &Window {
        self.surface.window()
    }
}

fn main() {

    // Step 1: Create Instance
    let instance = create_instance();
    let initializer = VulkanInit::create(&instance);


    // Needs to be mutable to poll events loop.
    let mut window_data = VulkanWindow::create(&initializer, instance.clone());

    // Step 5: Create vertex buffer
    vulkano::impl_vertex!(Vertex, position);

    let vertex1 = Vertex { position: [-0.9,  0.0] };
    let vertex2 = Vertex { position: [ 0.9,  0.0] };
    let vertex3 = Vertex { position: [ 0.0, -0.9] };

    let vertex4 = Vertex { position: [ 0.0,  0.9] };

    let vertex_buffer = CpuAccessibleBuffer::from_iter(initializer.device.clone(), 
                                                       BufferUsage::vertex_buffer(),
                                                       vec![vertex1, vertex2, vertex3, vertex4].into_iter()
                                                       ).unwrap();

    let index_buffer = CpuAccessibleBuffer::from_iter(initializer.device.clone(), BufferUsage::index_buffer(),
                                                      vec![0, 1, 2, 0, 3, 1].into_iter().map(|x| x as u32)).unwrap();



    mod vs {
        vulkano_shaders::shader!{
            ty: "vertex",
            src: "
    #version 450

    layout(location = 0) in vec2 position;
    layout(location = 0) out vec3 v_color;


    void main() {
        v_color = vec3(0.5*position + 0.5, 0.5);
        gl_Position = vec4(position, 0.0, 1.0);
    }
    "
        }
    }

    mod fs {

        vulkano_shaders::shader!{
            ty: "fragment",
            src: "
    #version 450
    layout(location = 0) in vec3 v_color;
    layout(location = 0) out vec4 f_color;

    void main() {
        f_color = vec4(v_color, 1.0);
    }
    "
        }
    }

    // Step 9: Load the shader
    let vs = vs::Shader::load(initializer.device.clone()).expect("failed to create shader module.");
    let fs = fs::Shader::load(initializer.device.clone()).expect("failed to create shader module.");


    // Step 11: Setup graphics pipeline
    let mut pipeline = Arc::new(GraphicsPipeline::start()
                                .vertex_input_single_buffer::<Vertex>()
                                .vertex_shader(vs.main_entry_point(), ())
                                .triangle_list()
                                .viewports_dynamic_scissors_irrelevant(1)
                                .viewports(iter::once(
                                        {
                                            Viewport { origin: [0.0, 0.0], 
                                                dimensions: [window_data.dimensions[0] as f32, window_data.dimensions[1] as f32],
                                                depth_range: 0.0 .. 1.0 }
                                        }
                                        )
                                          )
                                .fragment_shader(fs.main_entry_point(), ())
                                .render_pass(Subpass::from(window_data.render_pass.clone(), 0).unwrap())
                                .build(initializer.device.clone())
                                .unwrap()
                               );


    let mut recreate_swapchain = false;

    // Get a GPU Future pointer?
    let mut previous_frame_end = Box::new(sync::now(initializer.device.clone())) as Box<GpuFuture>;

    loop {
        // clean up previous frame.
        previous_frame_end.cleanup_finished();

        if recreate_swapchain {
            window_data.recreate_swapchain();
            //swapchain = new_swapchain;

            // Step 11: Setup graphics pipeline
            pipeline = Arc::new(GraphicsPipeline::start()
                                .vertex_input_single_buffer::<Vertex>()
                                .vertex_shader(vs.main_entry_point(), ())
                                .triangle_list()
                                .viewports_dynamic_scissors_irrelevant(1)
                                .viewports(iter::once( {
                                    Viewport { 
                                        origin: [0.0, 0.0], 
                                        dimensions: [window_data.dimensions[0] as f32, window_data.dimensions[1] as f32],
                                        depth_range: 0.0 .. 1.0 }
                                        }))
                                .fragment_shader(fs.main_entry_point(), ())
                                .render_pass(Subpass::from(window_data.render_pass.clone(), 0).unwrap())
                                .build(initializer.device.clone())
                                .unwrap()
                               );

            recreate_swapchain = false;

        }

        // Get the next image in the swapchain.
        let (image_num, acquire_future) = match swapchain::acquire_next_image(window_data.swapchain.clone(), None) {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => {
                recreate_swapchain = true;
                continue;
            }
            Err(err) => panic!("{:?}", err)
        };

        let clear_values = vec!([0.02, 0.02, 0.02, 1.0].into());

        // Create the command buffer for this frame
        let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(initializer.device.clone(), initializer.queue.family()).unwrap()
            .begin_render_pass(window_data.framebuffers[image_num].clone(), false, clear_values).unwrap()
            .draw_indexed(pipeline.clone(), &DynamicState::none(), vertex_buffer.clone(), index_buffer.clone(), (), ()).unwrap()
            .end_render_pass().unwrap()
            .build().unwrap();

        // Execute the commands in the command buffer, Present the image in the swapchain
        let future = previous_frame_end.join(acquire_future)
            .then_execute(initializer.queue.clone(), command_buffer).unwrap()
            .then_swapchain_present(initializer.queue.clone(), window_data.swapchain.clone(), image_num)
            .then_signal_fence_and_flush();

        // If it worked, the previous frame is now this frame. Otherwise, log the error and set the
        // previous frame ot a new frame.
        match future {
            Ok(future) => {
                previous_frame_end = Box::new(future) as Box<_>;
            }
            Err(FlushError::OutOfDate) => {
                recreate_swapchain = true;
                previous_frame_end = Box::new(sync::now(initializer.device.clone())) as Box<_>;
            }
            Err(e) => {
                println!("{:?}", e);
                previous_frame_end = Box::new(sync::now(initializer.device.clone())) as Box<_>;
            }
        }

        // Check if the user wants to close or resize the window.
        let mut done = false;
        window_data.events_loop.poll_events(|event| {
            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => done = true,
                Event::WindowEvent { event: WindowEvent::Resized(_), .. } => recreate_swapchain = true,
                _ => ()
            }
        });
        
        if done {
            return;
        }

    } 
}
