use std::sync::Arc;

use vulkano::command_buffer::*;
use vulkano::command_buffer::allocator::StandardCommandBufferAlloc;
use vulkano::image::{ImageUsage, ImageAccess};
use vulkano::image::{view::ImageView, SwapchainImage};
use vulkano::pipeline::PipelineBindPoint;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{LoadOp, StoreOp};
use vulkano::swapchain::{Swapchain as VkSwapchain, SwapchainCreateInfo, SwapchainPresentInfo, SwapchainAcquireFuture};

use super::pipeline::Pipeline;
use super::surface::Surface;
use super::device::Device;
use super::vertex::VisualiserVertexVec;
use super::sampler::Sampler;

struct Framebuffer {
	attachment_image: Arc<ImageView<SwapchainImage>>,
	command_buffer: Arc<PrimaryAutoCommandBuffer<StandardCommandBufferAlloc>>,
}

impl Framebuffer {
	fn create(
		image: Arc<SwapchainImage>,
		device: &Device,
		pipeline: &Pipeline,
		sampler: &Sampler,
		vertices: &VisualiserVertexVec,
		viewport: Viewport,
	) -> Self {
		let attachment_image = ImageView::new_default(image).unwrap();

		let command_buffer = Self::create_command_buffer(
			pipeline,
			device,
			&attachment_image,
			sampler,
			vertices,
			viewport
		);

		Self { attachment_image, command_buffer }
	}

	fn create_command_buffer(
		pipeline: &Pipeline,
		device: &Device,
		attachment_image: &Arc<ImageView<SwapchainImage>>,
		sampler: &Sampler,
		vertices: &VisualiserVertexVec,
		viewport: Viewport,
	) -> Arc<PrimaryAutoCommandBuffer<StandardCommandBufferAlloc>> {
		let mut builder = AutoCommandBufferBuilder::primary(
			&device.command_buffer_allocator,
			device.queue_family_index,
			CommandBufferUsage::MultipleSubmit,
		).unwrap();

		builder
		.copy_buffer_to_image(sampler.copy_operation()).unwrap()
		.begin_rendering(RenderingInfo {
			color_attachments: vec![Some(RenderingAttachmentInfo {
				load_op: LoadOp::Clear,
				store_op: StoreOp::Store,
				clear_value: Some([0.0; 4].into()),
				..RenderingAttachmentInfo::image_view(attachment_image.clone())
			})],
			..Default::default()
		}).unwrap()
		.set_viewport(0, [viewport])
		.bind_pipeline_graphics(pipeline.into())
		.bind_vertex_buffers(0, vertices.position_buffer(&device.memory_allocator))
		.bind_vertex_buffers(1, vertices.frequency_buffer(&device.memory_allocator))
		.bind_vertex_buffers(2, vertices.amplitude_buffer(&device.memory_allocator))
		.bind_descriptor_sets(
			PipelineBindPoint::Graphics,
			pipeline.layout(),
			0,
			vec![sampler.descriptor_set(device, pipeline.into())],
		)
		.draw(vertices.len() as u32, 1, 0, 0).unwrap()
		.end_rendering().unwrap();
		
		builder.build().map(Arc::new).unwrap()
	}
}

pub(crate) struct Swapchain {
	swapchain: Arc<VkSwapchain>,
	framebuffers: Box<[Framebuffer]>,
	pipeline: Pipeline,
	viewport: Viewport,
}

impl Swapchain {
	pub fn next(&self) -> (SwapchainAcquireFuture, SwapchainPresentInfo, Arc<PrimaryAutoCommandBuffer>) {
		let (index, suboptimal, acquire_future) = vulkano::swapchain::acquire_next_image(
			Arc::clone(&self.swapchain),
			None,
		).unwrap(); // TODO: handle AcquireError::OutOfDate

		let present_info = SwapchainPresentInfo::swapchain_image_index(Arc::clone(&self.swapchain), index);
		let command_buffer = Arc::clone(&self.framebuffers[index as usize].command_buffer);

		(acquire_future, present_info, command_buffer)
	}

	pub fn new(
		device: &Device,
		surface: &Surface,
		sampler: &Sampler,
		vertices: &VisualiserVertexVec,
		extent: [u32; 2],
	) -> Self {
		let (swapchain, swapchain_images) = VkSwapchain::new(
			device.into(),
			surface.into(),
			SwapchainCreateInfo {
				min_image_count: surface.framebuffer_count,
				image_format: Some(surface.format),
				image_extent: extent,
				image_usage: ImageUsage {
					color_attachment: true,
					.. ImageUsage::empty()
				},
				pre_transform: surface.transform,
				composite_alpha: surface.alpha_mode,
				..Default::default()
			},
		).expect("Failed to create swapchain");
		
		
		let mut viewport = Viewport {
			origin: [0.0, 0.0],
			dimensions: [0.0, 0.0],
			depth_range: 0.0..1.0,
		};
		
		let dimensions = swapchain_images.first().unwrap()
			.dimensions()
			.width_height();

		viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

		let pipeline = Pipeline::new(device.into(), surface.format);
	
		let framebuffers = swapchain_images.into_iter()
			.map(|image| Framebuffer::create(
				image,
				device,
				&pipeline,
				sampler,
				vertices,
				viewport.clone()
			))
			.collect::<Vec<_>>()
			.into_boxed_slice();
		
		Self { swapchain, framebuffers, pipeline, viewport }
	}
}