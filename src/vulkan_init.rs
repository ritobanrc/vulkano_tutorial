use std::sync::Arc;
use vulkano::instance::Instance;
use vulkano::instance::PhysicalDevice;
use vulkano::device::Device;
use vulkano::device::Queue;
use vulkano::device::DeviceExtensions;
use vulkano::device::Features;

pub struct VulkanInit<'a> {
    //instance: Arc<Instance>,
    pub physical: PhysicalDevice<'a>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>
}

impl<'a> VulkanInit<'a> {
    pub fn create(instance:&'a Arc<Instance>) -> VulkanInit<'a> {
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

