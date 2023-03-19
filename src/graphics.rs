use std::{sync::Arc, cmp::{max, min}};
use ahash::{HashMap, HashMapExt};

use wayland_client::protocol::wl_display::WlDisplay;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::Proxy;

use vulkano::{instance::*, sync::{self, FlushError}, pipeline::{PipelineBindPoint, Pipeline}, descriptor_set::{PersistentDescriptorSet, allocator::StandardDescriptorSetAllocator, WriteDescriptorSet}, sampler::{SamplerCreateInfo, Sampler}};
use vulkano::swapchain::*;
use vulkano::device::*;
use vulkano::device::physical::*;
use vulkano::image::*;
use vulkano::command_buffer::*;
use vulkano::command_buffer::allocator::*;
use vulkano::pipeline::graphics::vertex_input::*;
use vulkano::pipeline::graphics::viewport::*;
use vulkano::shader::*;
use vulkano::render_pass::*;
use vulkano::buffer::*;

use vulkano::VulkanLibrary;
use vulkano::image::view::ImageView;
use vulkano::sync::GpuFuture;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::render_pass::PipelineRenderingCreateInfo;
use vulkano::shader::spirv::ExecutionModel;
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::format::Format;
use vulkano::image::traits::ImageAccess;

use crate::BUFFER_SIZE;

const INSTANCE_EXTENSIONS: InstanceExtensions = InstanceExtensions {
	khr_surface: true,
	khr_wayland_surface: true,
	.. InstanceExtensions::empty()
};
	
const DEVICE_EXTENSIONS: DeviceExtensions = DeviceExtensions {
	khr_swapchain: true,
	.. DeviceExtensions::empty()
};

pub(crate) struct Graphics {
	instance: Arc<Instance>,
	device: Arc<Device>,
	queue: Arc<Queue>,
	surface: Arc<Surface>,
	swapchain: Arc<Swapchain>,
	swapchain_images: Vec<Arc<SwapchainImage>>,
	command_buffers: Vec<Arc<PrimaryAutoCommandBuffer<StandardCommandBufferAlloc>>>,
	pipeline: Arc<GraphicsPipeline>,
	previous_frame_future: Option<Box<dyn GpuFuture>>,
	visualiser_buffer: Arc<CpuAccessibleBuffer<[f32]>>,
}

struct ShaderSpecializations {}
unsafe impl SpecializationConstants for ShaderSpecializations {
    fn descriptors() -> &'static [SpecializationMapEntry] {
		static DESCRIPTORS: [SpecializationMapEntry; 0] = [];

		&DESCRIPTORS
    }
}

impl Graphics {
	fn create_vertex_shader(device: Arc<Device>) -> Arc<ShaderModule> {
		unsafe {
			ShaderModule::from_bytes(device, include_bytes!("shaders/vert.spv"))
				.unwrap()
		}
	}

	fn create_fragment_shader(device: Arc<Device>) -> Arc<ShaderModule> {
		unsafe {
			ShaderModule::from_bytes(device, include_bytes!("shaders/frag.spv"))
				.unwrap()
		}
	}

	fn create_pipeline(
		device: Arc<Device>,
		format: Format,
		swapchain: &Arc<Swapchain>,
	) -> Arc<GraphicsPipeline> {
		let vs = Self::create_vertex_shader(Arc::clone(&device));
		let fs = Self::create_fragment_shader(Arc::clone(&device));
		let vertex = vs.entry_point_with_execution("main", ExecutionModel::Vertex).unwrap();
		let fragment = fs.entry_point_with_execution("main", ExecutionModel::Fragment).unwrap();

		let mut vertex_bindings = HashMap::new();
		vertex_bindings.insert(0, VertexInputBindingDescription {
			stride: 8,
			input_rate: VertexInputRate::Vertex,
		});
		vertex_bindings.insert(1, VertexInputBindingDescription {
			stride: 4,
			input_rate: VertexInputRate::Vertex,
		});
		vertex_bindings.insert(2, VertexInputBindingDescription {
			stride: 4,
			input_rate: VertexInputRate::Vertex,
		});

		let mut vertex_attributes = HashMap::new();
		vertex_attributes.insert(0, VertexInputAttributeDescription {
			binding: 0,
			format: Format::R32G32_SFLOAT,
			offset: 0,
		});
		vertex_attributes.insert(1, VertexInputAttributeDescription {
			binding: 1,
			format: Format::R32_SFLOAT,
			offset: 0,
		});
		vertex_attributes.insert(2, VertexInputAttributeDescription {
			binding: 2,
			format: Format::R32_SFLOAT,
			offset: 0,
		});

		GraphicsPipeline::start()
			.vertex_shader(vertex, ShaderSpecializations {})
			.fragment_shader(fragment, ShaderSpecializations {})
			.render_pass(PipelineRenderingCreateInfo {
				color_attachment_formats: vec![Some(swapchain.image_format())],
				..Default::default()
			})
			.vertex_input_state(VertexInputState {
				bindings: vertex_bindings,
				attributes: vertex_attributes,
			})
			.input_assembly_state(InputAssemblyState::new())
			.viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
			.build(device)
			.unwrap()
	}

	fn create_command_buffer(
		allocator: &StandardCommandBufferAllocator,
		queue: Arc<Queue>,
		image: Arc<ImageView<SwapchainImage>>,
		pipeline: Arc<GraphicsPipeline>,
		descriptor_set: Arc<PersistentDescriptorSet>,
		vertices: Arc<CpuAccessibleBuffer<[[f32; 2]]>>,
		frequency: Arc<CpuAccessibleBuffer<[f32]>>,
		amplitude: Arc<CpuAccessibleBuffer<[f32]>>,
		visualizer_buffer: Arc<CpuAccessibleBuffer<[f32]>>,
		visualizer_data: Arc<StorageImage>,
	) -> Arc<PrimaryAutoCommandBuffer<StandardCommandBufferAlloc>> {
		let mut builder = AutoCommandBufferBuilder::primary(
			allocator,
			queue.queue_family_index(),
			CommandBufferUsage::MultipleSubmit,
		).unwrap();

		let vertex_count = vertices.len();

		let layout = Arc::clone(pipeline.layout());

		builder
		.copy_buffer_to_image(
			CopyBufferToImageInfo::buffer_image(
				visualizer_buffer,
				visualizer_data,
			)
		).unwrap()
		.begin_rendering(RenderingInfo {
			color_attachments: vec![Some(RenderingAttachmentInfo {
				load_op: LoadOp::Clear,
				store_op: StoreOp::Store,
				clear_value: Some([0.0; 4].into()),
				..RenderingAttachmentInfo::image_view(image)
			})],
			..Default::default()
		}).unwrap()
		.set_viewport(0, [Viewport {
			origin: [0.0; 2],
			dimensions: [320.0, 240.0],
			depth_range: 0.0..1.0
		}])
		.bind_pipeline_graphics(pipeline)
		.bind_vertex_buffers(0, vertices)
		.bind_vertex_buffers(1, frequency)
		.bind_vertex_buffers(2, amplitude)
		.bind_descriptor_sets(
			PipelineBindPoint::Graphics,
			layout,
			0,
			vec![descriptor_set],
		)
		.draw(vertex_count as u32, 1, 0, 0).unwrap()
		.end_rendering().unwrap();
		
		builder.build().map(Arc::new).unwrap()
	}

	fn choose_device(
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
						let supports_surface = device.surface_support(i as u32, &surface).unwrap_or(false);
						
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

	fn instance() -> Arc<Instance> {
		let library = VulkanLibrary::new().expect("Failed to load vulkan library");

		Instance::new(
			library,
			InstanceCreateInfo {
				enabled_extensions: INSTANCE_EXTENSIONS,
				.. Default::default()
			}
		).expect("Couldn't build instance")
	}

	fn window_size_dependent_setup(
		images: &[Arc<SwapchainImage>],
		viewport: &mut Viewport,
	) -> Vec<Arc<ImageView<SwapchainImage>>> {
		let dimensions = images[0].dimensions().width_height();
		viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];
	
		images
			.iter()
			.map(|image| ImageView::new_default(image.clone()).unwrap())
			.collect::<Vec<_>>()
	}

	pub fn new(
		display: &WlDisplay,
		surface: &WlSurface,
		extent: [u32; 2],
	) -> Self {
		let instance = Self::instance();

		let display_pointer = display.id().as_ptr();
		let surface_pointer = surface.id().as_ptr();
		//let data_holder = Arc::new((display, surface));

		let surface = unsafe {
			Surface::from_wayland(
				Arc::clone(&instance),
				display_pointer, 
				surface_pointer,
				None,
			)
		}.expect("Failed to create vulkan surface");

		let (physical_device, queue_family_index) = Self::choose_device(&instance, &surface)
			.expect("No suitable graphics device");

		let (device, mut queue) = Device::new(
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

		let queue = queue.next().expect("Device queue was empty");

		let surface_capabilities = device
			.physical_device()
			.surface_capabilities(&surface, Default::default())
			.unwrap();

		let min_image_count = match surface_capabilities.max_image_count {
			None => max(2, surface_capabilities.min_image_count),
			Some(limit) => min(max(2, surface_capabilities.min_image_count), limit)
		};

		let formats = device.physical_device()
			.surface_formats(&surface, Default::default())
			.unwrap();

		let format = formats.into_iter()
			.map(|f| f.0)
			.find(|format| {
				matches!(format, Format::B8G8R8A8_SRGB
					| Format::B8G8R8A8_UNORM
					| Format::R8G8B8A8_SRGB
					| Format::R8G8B8A8_UNORM
				)
			}).expect("Found no suitable image format");
		
		let (swapchain, swapchain_images) = Swapchain::new(
			Arc::clone(&device),
			Arc::clone(&surface),
			SwapchainCreateInfo {
				min_image_count,
				image_format: Some(format),
				image_extent: extent,
				image_usage: ImageUsage {
					color_attachment: true,
					.. ImageUsage::empty()
				},
				pre_transform: surface_capabilities.current_transform,
				composite_alpha: surface_capabilities.supported_composite_alpha	
					.iter()
					.find(|mode| matches!(mode, CompositeAlpha::PreMultiplied | CompositeAlpha::PostMultiplied))
					.expect("Transparent windows not supported"),
				..Default::default()
			},
		).expect("Failed to create swapchain");

		let mut viewport = Viewport {
			origin: [0.0, 0.0],
			dimensions: [0.0, 0.0],
			depth_range: 0.0..1.0,
		};
		
		let mut attachment_image_views = Self::window_size_dependent_setup(
			&swapchain_images,
			&mut viewport,
		);

		let pipeline = Self::create_pipeline(Arc::clone(&device), format, &swapchain);
		
		let command_buffer_allocator = StandardCommandBufferAllocator::new(
			device.clone(),
			Default::default(),
		);

		let memory_allocator = StandardMemoryAllocator::new_default(device.clone());

		let visualiser_data = StorageImage::new(
			&memory_allocator,
			ImageDimensions::Dim1d {
				width: BUFFER_SIZE as u32,
				array_layers: 1,
			},
			Format::D32_SFLOAT,
			[queue.queue_family_index()],
		).unwrap();

		let visualiser_buffer = CpuAccessibleBuffer::from_iter(
			&memory_allocator,
			BufferUsage {
				transfer_src: true,
				..BufferUsage::empty()
			},
			true,
			[0.5f32; BUFFER_SIZE].into_iter(),
		).unwrap();

		let texture = ImageView::new_default(Arc::clone(&visualiser_data)).unwrap();

		let sampler = Sampler::new(
			Arc::clone(&device),
			SamplerCreateInfo::simple_repeat_linear_no_mipmap()
		).unwrap();

		let descriptor_set = PersistentDescriptorSet::new(
			&StandardDescriptorSetAllocator::new(Arc::clone(&device)),
			Arc::clone(pipeline.layout().set_layouts().get(0).unwrap()),
			[WriteDescriptorSet::image_view_sampler(0, texture, sampler)],
		).unwrap();

		let vertex_position = CpuAccessibleBuffer::from_iter(
			&memory_allocator,
			BufferUsage {
				vertex_buffer: true,
				..BufferUsage::empty()
			},
			false,
			[
				[0.0, 0.0],
				[1.0, 0.0],
				[0.0, 1.0],
			],
		).unwrap();

		let vertex_frequency = CpuAccessibleBuffer::from_iter(
			&memory_allocator,
			BufferUsage {
				vertex_buffer: true,
				..BufferUsage::empty()
			},
			false,
			[
				0.0,
				0.0,
				0.0,
			],
		).unwrap();

		let vertex_amplitude = CpuAccessibleBuffer::from_iter(
			&memory_allocator,
			BufferUsage {
				vertex_buffer: true,
				..BufferUsage::empty()
			},
			false,
			[
				0.0,
				1.0,
				0.0,
			],
		).unwrap();

		let command_buffers = attachment_image_views.iter()
			.map(|image| Self::create_command_buffer(
				&command_buffer_allocator,
				Arc::clone(&queue),
				Arc::clone(image),
				Arc::clone(&pipeline),
				Arc::clone(&descriptor_set),
				Arc::clone(&vertex_position),
				Arc::clone(&vertex_frequency),
				Arc::clone(&vertex_amplitude),
				Arc::clone(&visualiser_buffer),
				Arc::clone(&visualiser_data),
			))
			.collect();

		let previous_frame_future = Some(sync::now(Arc::clone(&device)).boxed());

		Graphics {
			instance,
			device,
			queue,
			surface,
			swapchain,
			swapchain_images,
			command_buffers,
			pipeline,
			previous_frame_future,
			visualiser_buffer,
		}
	}

	pub fn draw(&mut self, buffer: &[f32; BUFFER_SIZE]) {
		let mut previous_future = self.previous_frame_future.take()
			.unwrap_or_else(|| sync::now(Arc::clone(&self.device)).boxed());

		previous_future.cleanup_finished();

		match self.visualiser_buffer.write() {
			Ok(mut visualiser) => visualiser.copy_from_slice(buffer),
			Err(_) => {
				self.previous_frame_future = Some(previous_future);
				// if we can't change the buffer then the frame would be the same
				return;
			}
		}

		let (index, suboptimal, acquire_future) = vulkano::swapchain::acquire_next_image(
			Arc::clone(&self.swapchain),
			None,
		).unwrap(); // TODO: handle AcquireError::OutOfDate

		let future = previous_future
			.join(acquire_future)
			.then_execute(
				Arc::clone(&self.queue),
				Arc::clone(&self.command_buffers[index as usize]),
			).unwrap()
			.then_swapchain_present(
				Arc::clone(&self.queue),
				SwapchainPresentInfo::swapchain_image_index(Arc::clone(&self.swapchain), index),
			)
			.then_signal_fence_and_flush();
		
		match future {
			Ok(future) => {
				self.previous_frame_future = Some(future.boxed())
			},
			Err(FlushError::OutOfDate) => {
				todo!()
			},
			Err(e) => {
				todo!("{:?}", e)
			}
		}
	}
}

