use std::sync::Arc;

use wayland_client::protocol::wl_display::WlDisplay;
use wayland_client::protocol::wl_surface::WlSurface;

use vulkano::{instance::*, sync::{self, FlushError}};
use vulkano::shader::*;

use vulkano::VulkanLibrary;

use vulkano::sync::GpuFuture;

use crate::BUFFER_SIZE;

use self::{surface::Surface, swapchain::Swapchain, vertex::{VisualiserVertex,VisualiserVertexVec}, sampler::Sampler, device::Device};

mod swapchain;
mod surface;
mod pipeline;
mod device;

mod vertex;
mod sampler;

const INSTANCE_EXTENSIONS: InstanceExtensions = InstanceExtensions {
	khr_surface: true,
	khr_wayland_surface: true,
	.. InstanceExtensions::empty()
};

pub(crate) struct Graphics {
	instance: Arc<Instance>,
	device: Device,
	surface: Surface,
	swapchain: Swapchain,
	visualiser_sampler: Sampler,
	previous_frame_future: Option<Box<dyn GpuFuture>>,
}

struct ShaderSpecializations {}
unsafe impl SpecializationConstants for ShaderSpecializations {
    fn descriptors() -> &'static [SpecializationMapEntry] {
		static DESCRIPTORS: [SpecializationMapEntry; 0] = [];

		&DESCRIPTORS
    }
}

impl Graphics {
	fn instance() -> Arc<Instance> {
		let library = VulkanLibrary::new()
			.expect("Failed to load vulkan library");

		Instance::new(
			library,
			InstanceCreateInfo {
				enabled_extensions: INSTANCE_EXTENSIONS,
				.. Default::default()
			}
		).expect("Couldn't build instance")
	}

	pub fn new(
		display: &WlDisplay,
		surface: &WlSurface,
		extent: [u32; 2],
	) -> Self {
		let instance = Self::instance();

		let (surface, device) = Surface::from_wayland(Arc::clone(&instance), display, surface);
		
		let visualiser_sampler = Sampler::new(&device);

		let mut vertices = VisualiserVertexVec::with_capacity(6);

		vertices.push(VisualiserVertex { position: [-1.0, -1.0], frequency: 0.0, amplitude: 1.0 });
		vertices.push(VisualiserVertex { position: [1.0, -1.0], frequency: 1.0, amplitude: 1.0 });
		vertices.push(VisualiserVertex { position: [1.0, 1.0], frequency: 1.0, amplitude: 0.0 });
		vertices.push(VisualiserVertex { position: [-1.0, -1.0], frequency: 0.0, amplitude: 1.0 });
		vertices.push(VisualiserVertex { position: [-1.0, 1.0], frequency: 0.0, amplitude: 0.0 });
		vertices.push(VisualiserVertex { position: [1.0, 1.0], frequency: 1.0, amplitude: 0.0 });
		
		let swapchain = Swapchain::new(&device, &surface, &visualiser_sampler, &vertices, extent);

		let previous_frame_future = Some(sync::now((&device).into()).boxed());

		Graphics {
			instance,
			device,
			surface,
			swapchain,
			previous_frame_future,
			visualiser_sampler,
		}
	}

	pub fn draw(&mut self, buffer: Option<Box<[f32; BUFFER_SIZE]>>) {
		let mut previous_future = self.previous_frame_future.take()
			.unwrap_or_else(|| sync::now((&self.device).into()).boxed());

		previous_future.cleanup_finished();

		// If data is none, we don't need to update the surface.
		// However, wayland will not send the next frame callback until we do.
		// So, we draw anyway.
		if let Some(data) = buffer {
			match self.visualiser_sampler.buffer.write() {
				Ok(mut visualiser) => visualiser.copy_from_slice(data.as_slice()),
				Err(_) => {
					self.previous_frame_future = Some(previous_future);
					// if we can't change the buffer then the frame would be the same
					return;
				}
			}
		}

		let (acquire_future, present_info, command_buffer) = self.swapchain.next();

		let future = previous_future
			.join(acquire_future)
			.then_execute(
				Arc::clone(&self.device.queue),
				command_buffer,
			).unwrap()
			.then_swapchain_present(
				Arc::clone(&self.device.queue),
				present_info,
			)
			.then_signal_fence_and_flush();

		match future {
			Ok(future) => {
				self.previous_frame_future = Some(future.boxed());
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

