mod vulkan_init;
mod vulkan_window;

use std::sync::Arc;
use std::iter;
use vulkano::instance::Instance;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::framebuffer::Subpass;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::buffer::BufferUsage;
use vulkano::swapchain;
use vulkano::swapchain::AcquireError;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::DynamicState;
use vulkano::pipeline::viewport::Viewport;
use vulkano::sync;
use vulkano::sync::{GpuFuture, FlushError};
use winit::Event;
use winit::WindowEvent;

use vulkan_window::VulkanWindow;
use vulkan_init::VulkanInit;


struct Vertex {
    position: [f32; 2], 
}

fn create_instance() -> Arc<Instance> {
    // TODO: generalize this to not rendering to a window
    let extentions = vulkano_win::required_extensions();
    Instance::new(None, &extentions, None).expect("Failed to create instance.")
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
