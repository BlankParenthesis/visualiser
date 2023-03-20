use std::sync::Arc;

use vulkano::{image::{StorageImage, ImageDimensions, view::ImageView}, descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet}, sampler::{Sampler as VkSampler, SamplerCreateInfo}, format::Format, buffer::{BufferUsage, CpuAccessibleBuffer}, command_buffer::CopyBufferToImageInfo, pipeline::{GraphicsPipeline, Pipeline}};

use crate::BUFFER_SIZE;

use super::device::Device;

pub(crate) struct Sampler {
	sampler: Arc<VkSampler>,
	pub buffer: Arc<CpuAccessibleBuffer<[f32]>>,
	image_view: Arc<ImageView<StorageImage>>,
}

impl Sampler {
	pub fn new(device: &Device) -> Self {
		let buffer = CpuAccessibleBuffer::from_iter(
			&device.memory_allocator,
			BufferUsage {
				transfer_src: true,
				..BufferUsage::empty()
			},
			true,
			[f32::default(); BUFFER_SIZE].into_iter(),
		).unwrap();
		
		let image = StorageImage::new(
			&device.memory_allocator,
			ImageDimensions::Dim1d {
				width: BUFFER_SIZE as u32,
				array_layers: 1,
			},
			Format::R32_SFLOAT,
			[device.queue_family_index],
		).unwrap();

		let image_view = ImageView::new_default(image).unwrap();

		let sampler = VkSampler::new(
			device.into(),
			SamplerCreateInfo::simple_repeat_linear_no_mipmap()
		).unwrap();

		Self { sampler, buffer, image_view }
	}

	pub fn descriptor_set(&self, device: &Device, pipeline: Arc<GraphicsPipeline>) -> Arc<PersistentDescriptorSet> {
		PersistentDescriptorSet::new(
			&device.descriptor_allocator,
			Arc::clone(pipeline.layout().set_layouts().first().unwrap()),
			[WriteDescriptorSet::image_view_sampler(0, self.image_view.clone(), Arc::clone(&self.sampler))],
		).unwrap()
	}

	pub fn copy_operation(&self) -> CopyBufferToImageInfo {
		CopyBufferToImageInfo::buffer_image(
			self.buffer.clone(),
			self.image_view.image().clone(),
		)
	}
}