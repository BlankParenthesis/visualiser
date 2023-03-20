use std::sync::Arc;

use vulkano::{command_buffer::PrimaryAutoCommandBuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAlloc;
use vulkano::format::Format;
use vulkano::device::Device;
use vulkano::shader::ShaderModule;
use vulkano::pipeline::{GraphicsPipeline, Pipeline as VkPipeline, PipelineLayout};
use vulkano::shader::spirv::ExecutionModel;
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::graphics::render_pass::PipelineRenderingCreateInfo;

use super::vertex::Vertex;
use super::{vertex::VisualiserVertex, ShaderSpecializations};

pub(crate) struct Pipeline {
	pipeline: Arc<GraphicsPipeline>,
	format: Format,
}

impl From<&Pipeline> for Arc<GraphicsPipeline> {
	fn from(pipeline: &Pipeline) -> Self {
		Arc::clone(&pipeline.pipeline)
	}
}

impl Pipeline {
	pub fn layout(&self) -> Arc<PipelineLayout> {
		Arc::clone(&self.pipeline.layout())
	}

	pub fn new(device: Arc<Device>, format: Format) -> Self {
		let vs = unsafe {
			ShaderModule::from_bytes(
				Arc::clone(&device),
				include_bytes!("../shaders/vert.spv")
			).unwrap()
		};

		let fs = unsafe {
			ShaderModule::from_bytes(
				Arc::clone(&device),
				include_bytes!("../shaders/frag.spv")
			).unwrap()
		};

		let vertex = vs.entry_point_with_execution("main", ExecutionModel::Vertex).unwrap();
		let fragment = fs.entry_point_with_execution("main", ExecutionModel::Fragment).unwrap();

		let pipeline = GraphicsPipeline::start()
			.vertex_shader(vertex, ShaderSpecializations {})
			.fragment_shader(fragment, ShaderSpecializations {})
			.render_pass(PipelineRenderingCreateInfo {
				color_attachment_formats: vec![Some(format)],
				..Default::default()
			})
			.vertex_input_state(VisualiserVertex::input_state())
			.viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
			.build(device).unwrap();

		Self { pipeline, format }
	}
}