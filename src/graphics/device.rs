use std::sync::Arc;

use vulkano::{device::{Device as VkDevice, DeviceCreateInfo, QueueCreateInfo, Features, DeviceExtensions, physical::{PhysicalDevice, PhysicalDeviceType, PhysicalDeviceError}, Queue}, instance::Instance, swapchain::{Surface, SurfaceCapabilities}, memory::allocator::StandardMemoryAllocator, descriptor_set::allocator::StandardDescriptorSetAllocator, command_buffer::allocator::StandardCommandBufferAllocator};

pub(crate) struct Device {
	device: Arc<VkDevice>,
	pub queue_family_index: u32,
	pub queue: Arc<Queue>,

	pub memory_allocator: StandardMemoryAllocator,
	pub descriptor_allocator: StandardDescriptorSetAllocator,
	pub command_buffer_allocator: StandardCommandBufferAllocator,
}

impl From<&Device> for Arc<VkDevice> {
	fn from(device: &Device) -> Self {
		Arc::clone(&device.device)
	}
}

const DEVICE_EXTENSIONS: DeviceExtensions = DeviceExtensions {
	khr_swapchain: true,
	.. DeviceExtensions::empty()
};

impl Device {
	pub fn new(instance: &Arc<Instance>, surface: &Arc<Surface>) -> Self {
		let (physical_device, queue_family_index) = Self::choose_device(instance, surface)
			.expect("No suitable graphics device");

		let (device, mut queues) = VkDevice::new(
			physical_device,
			DeviceCreateInfo {
				queue_create_infos: vec![QueueCreateInfo {
					queue_family_index,
					..Default::default()
				}],
				enabled_extensions: DEVICE_EXTENSIONS,
				enabled_features: Features {
					dynamic_rendering: true,
					..Features::empty()
				},
				..Default::default()
			}
		).expect("Failed to create graphics device");

		let queue = queues.next().unwrap();
		
		let memory_allocator = StandardMemoryAllocator::new_default(Arc::clone(&device));
		let descriptor_allocator = StandardDescriptorSetAllocator::new(Arc::clone(&device));
		let command_buffer_allocator = StandardCommandBufferAllocator::new(Arc::clone(&device), Default::default());
		
		Self {
			device,
			queue_family_index,
			queue,
			memory_allocator,
			descriptor_allocator,
			command_buffer_allocator,
		}
	}

	pub fn choose_device(
		instance: &Arc<Instance>,
		surface: &Arc<Surface>,
	) -> Option<(Arc<PhysicalDevice>, u32)> {
		instance.enumerate_physical_devices()
			.unwrap()
			.filter(|device| device.supported_extensions().contains(&DEVICE_EXTENSIONS))
			.filter_map(|device| {
				device.queue_family_properties()
					.iter()
					.enumerate()
					.position(|(i, queue_properties)| {
						let supports_graphics = queue_properties.queue_flags.graphics;
						let supports_surface = device.surface_support(i as u32, surface).unwrap_or(false);
						
						supports_graphics && supports_surface
					})
					.map(|i| (device, i as u32))
			})
			.min_by_key(|(device, _)| {
				match device.properties().device_type {
					PhysicalDeviceType::DiscreteGpu => 0,
					PhysicalDeviceType::IntegratedGpu => 1,
					PhysicalDeviceType::VirtualGpu => 2,
					PhysicalDeviceType::Cpu => 3,
					PhysicalDeviceType::Other => 4,
					_ => 5,
				}
			})
	}

	pub fn physical_device(&self) -> &PhysicalDevice {
		self.device.physical_device().as_ref()
	}
}