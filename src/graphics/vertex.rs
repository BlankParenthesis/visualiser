use std::sync::Arc;

use ahash::{HashMap, HashMapExt};

use soa_derive::StructOfArray;
use vulkano::buffer::{CpuAccessibleBuffer, BufferUsage};
use vulkano::memory::allocator::MemoryAllocator;
use vulkano::pipeline::graphics::vertex_input::*;
use vulkano::format::Format;

#[derive(StructOfArray)]
pub(crate) struct VisualiserVertex {
	pub position: [f32; 2],
	pub frequency: f32,
	pub amplitude: f32,
}

impl VisualiserVertexVec {
	pub fn position_buffer(
		&self,
		memory_allocator: &impl MemoryAllocator,
	) -> Arc<CpuAccessibleBuffer<[[f32; 2]]>> {
		CpuAccessibleBuffer::from_iter(
			memory_allocator,
			BufferUsage {
				vertex_buffer: true,
				..BufferUsage::empty()
			},
			false,
			self.position.clone(),
		).unwrap()
	}

	pub fn frequency_buffer(
		&self,
		memory_allocator: &impl MemoryAllocator,
	) -> Arc<CpuAccessibleBuffer<[f32]>> {
		CpuAccessibleBuffer::from_iter(
			memory_allocator,
			BufferUsage {
				vertex_buffer: true,
				..BufferUsage::empty()
			},
			false,
			self.frequency.clone(),
		).unwrap()
	}

	pub fn amplitude_buffer(
		&self,
		memory_allocator: &impl MemoryAllocator,
	) -> Arc<CpuAccessibleBuffer<[f32]>> {
		CpuAccessibleBuffer::from_iter(
			memory_allocator,
			BufferUsage {
				vertex_buffer: true,
				..BufferUsage::empty()
			},
			false,
			self.amplitude.clone(),
		).unwrap()
	}
}

pub(crate) trait Vertex {
	fn input_state() -> VertexInputState;
	//fn attributes(index: u32) -> Iterator<Item = >;
}

impl Vertex for VisualiserVertex {
    fn input_state() -> VertexInputState {
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

		VertexInputState {
			bindings: vertex_bindings,
			attributes: vertex_attributes,
		}
    }
}