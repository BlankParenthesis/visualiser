use std::sync::Arc;

use vulkano::instance::Instance;
use wayland_client::Proxy;
use wayland_client::protocol::wl_display::WlDisplay;
use wayland_client::protocol::wl_surface::WlSurface;

use vulkano::format::Format;
use vulkano::swapchain::{Surface as VkSurface, *};

use super::device::Device;

pub(crate) struct Surface {
	pub surface: Arc<VkSurface>,
	pub format: Format,
	pub alpha_mode: CompositeAlpha,
	pub transform: SurfaceTransform,
	pub framebuffer_count: u32,
}

impl From<&Surface> for Arc<VkSurface> {
    fn from(surface: &Surface) -> Self {
        Arc::clone(&surface.surface)
    }
}

impl Surface {
	pub fn from_wayland(
		instance: Arc<Instance>,
		display: &WlDisplay,
		surface: &WlSurface,
	) -> (Self, Device) {
		let display_pointer = display.id().as_ptr();
		let surface_pointer = surface.id().as_ptr();

		let surface = unsafe {
			VkSurface::from_wayland(
				Arc::clone(&instance),
				display_pointer, 
				surface_pointer,
				None,
			)
		}.expect("Failed to create vulkan surface");

		let device = Device::new(&instance, &surface);

		let capabilities = device.physical_device()
			.surface_capabilities(&surface, Default::default())
			.expect("Device failed to provide surface capabilities");

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
			})
			.expect("Failed to find suitable format for surface");

		let alpha_mode = capabilities.supported_composite_alpha.iter()
				.find(|mode| {
					use CompositeAlpha::*;
		
					matches!(mode, PreMultiplied | PostMultiplied)
				})
				.expect("Surface does not support transparency");

		let transform = capabilities.current_transform;

		let framebuffer_count = capabilities.min_image_count;

		let surface = Self {
			surface,
			format,
			alpha_mode,
			transform,
			framebuffer_count,
		};

		(surface, device)
	}
}