use std::sync::Arc;
use vulkano::instance::Instance;
use vulkano::instance::InstanceExtensions;
use vulkano::instance::PhysicalDevice;
use vulkano::device::Device;
use vulkano::device::DeviceExtensions;
use vulkano::device::Features;
use vulkano::buffer::BufferUsage;
use vulkano::buffer::CpuAccessibleBuffer;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::command_buffer::CommandBuffer;
use vulkano::sync::GpuFuture;
use vulkano::pipeline::ComputePipeline;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::format::Format;
use vulkano::image::Dimensions;
use vulkano::image::StorageImage;
use vulkano::format::ClearValue;
use image::{ImageBuffer, Rgba};

mod cs {
    vulkano_shaders::shader!{
        ty: "compute",
        src: "

#version 450

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;

void main() {
    vec2 norm_coordinates = (gl_GlobalInvocationID.xy + vec2(0.5)) / vec2(imageSize(img));
    vec2 c = (norm_coordinates - vec2(0.5)) * 3.0 - vec2(0.5, 0.0);

    vec2 z = vec2(0.0, 0.0);
    float i;
    for (i = 0.0; i < 1.0; i += 0.05) {
        z = vec2(
            z.x * z.x - z.y * z.y + c.x,
            z.y * z.x + z.x * z.y + c.y
        );

        if (length(z) > 4.0) {
            break;
        }
    }

    vec4 to_write = vec4(i, 0.3 - i, 0.0, 1.0);
    imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
}
"
    }
}

fn main() {
    let instance = Instance::new(None, &InstanceExtensions::none(), None)
        .expect("Failed to create instance.");

    let physical = PhysicalDevice::enumerate(&instance).next().expect("No physical devices available.");

    for family in physical.queue_families() {
        println!("Found a queue family with {:?} queues", family.queues_count());
    }

    let queue_family =   physical.queue_families()
        .find(|&q| q.supports_graphics())
        .expect("Couldn't find a graphical queue family.");

    let (device, mut queues) = {
        Device::new(physical, &Features::none(), &DeviceExtensions::none(), 
                    [(queue_family, 0.5)].iter().cloned())
            .expect("Failed to create device.")
    };

    let queue = queues.next().unwrap();

    /*
    let source_content = 0..64;
    let source = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), source_content)
        .expect("Failed to create source buffer.");

    let dest_content = (0..64).map(|_| 0);
    let dest = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), dest_content)
        .expect("Failed to created destination buffer.");

    let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap()
        .copy_buffer(source.clone(), dest.clone()).unwrap()
        .build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();
    finished.then_signal_fence_and_flush().unwrap()
        .wait(None).unwrap();

    let src_content = source.read().unwrap();
    let dest_content = dest.read().unwrap();

    assert_eq!(&*src_content, &*dest_content)
    */

    /*
    let data_iter = 0..65536;
    let data_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), data_iter).unwrap();

    let shader = cs::Shader::load(device.clone()).unwrap();
    let compute_pipeline = Arc::new(ComputePipeline::new(device.clone(), &shader.main_entry_point(), &())
                                    .expect("Failed to create compute pipeline"));

    let set = Arc::new(PersistentDescriptorSet::start(compute_pipeline.clone(), 0)
                       .add_buffer(data_buffer.clone()).unwrap()
                       .build().unwrap()
                      );

    let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap()
        .dispatch([1024, 1, 1], compute_pipeline.clone(), set.clone(), ()).unwrap()
        .build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();
    finished.then_signal_fence_and_flush().unwrap()
        .wait(None).unwrap();

    let content = data_buffer.read().unwrap();
    for (n, val) in content.iter().enumerate() {
        assert_eq!(*val, n as u32 * 12);
    }

    println!("Success!")
    */

    let image = StorageImage::new(device.clone(), 
                                  Dimensions::Dim2d{ width: 1024, height: 1024 },
                                  Format::R8G8B8A8Unorm,
                                  Some(queue.family())).unwrap();


    let shader = cs::Shader::load(device.clone()).unwrap();
    let compute_pipeline = Arc::new(ComputePipeline::new(device.clone(), &shader.main_entry_point(), &())
                                    .expect("Failed to create compute pipeline"));

    let set = Arc::new(PersistentDescriptorSet::start(compute_pipeline.clone(), 0)
                       .add_image(image.clone()).unwrap()
                       .build().unwrap()
                      );

    let buf = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), 
                                             (0 .. 1024 * 1024 * 4).map(|_| 0u8))
        .expect("Failed to create CpuAccessibleBuffer");

    let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap()
        .dispatch([1024/8, 1024/8, 1], compute_pipeline.clone(), set.clone(), ()).unwrap()
        .copy_image_to_buffer(image.clone(), buf.clone()).unwrap()
        .build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();
    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();

    let buffer_content = buf.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();

    image.save("image.png").unwrap();

}

