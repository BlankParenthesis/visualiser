use std::sync::{RwLock, Arc};
use std::time::Duration;

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, EventQueue};
use wayland_client::protocol::wl_surface::{self, WlSurface};
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_compositor::{self, WlCompositor};
use wayland_client::protocol::wl_callback::{self, WlCallback};
use wayland_client::protocol::wl_display::WlDisplay;
use wayland_client::protocol::wl_region::{self, WlRegion};
use wayland_protocols::xdg::shell::client::xdg_surface::{self, XdgSurface};
use wayland_protocols::xdg::shell::client::xdg_toplevel::{self, XdgToplevel};
use wayland_protocols::xdg::shell::client::xdg_wm_base::{self, XdgWmBase};

use crate::BUFFER_SIZE;
use crate::graphics::Graphics;
use crate::visualiser::BufferManager;

struct GraphicsState {
	surface: XdgSurface,
	toplevel: XdgToplevel,
	graphics: Graphics,
}

pub(crate) struct Window {
	running: bool,
	display: WlDisplay,
	event_queue: Option<EventQueue<Self>>,
	wm_base: Option<XdgWmBase>,
	base_surface: Option<WlSurface>,
	graphics_state: Option<GraphicsState>,
	configured: bool,
	visualiser: Arc<RwLock<BufferManager>>,
	last_frame: u32,
}

impl Window {
	pub fn new(visualiser: Arc<RwLock<BufferManager>>) -> Self {
		let connection = Connection::connect_to_env().unwrap();
	
		let event_queue = connection.new_event_queue();
		let queue_handle = event_queue.handle();
		
		let display = connection.display();
		display.get_registry(&queue_handle, ());
	
		Window {
			running: false,
			display,
			event_queue: Some(event_queue),
			wm_base: None,
			base_surface: None,
			graphics_state: None,
			configured: false,
			visualiser,
			last_frame: 0,
		}
	}

	pub fn run(&mut self) {
		self.running = true;

		let mut event_queue = self.event_queue.take().unwrap();

		while self.running {
			event_queue.blocking_dispatch(self).unwrap();
		}
	}

	fn init_xdg_surface(&mut self, queue_handle: &QueueHandle<Window>) {
		let wm_base = self.wm_base.as_ref().unwrap();
		let base_surface = self.base_surface.as_ref().unwrap();

		let xdg_surface = wm_base.get_xdg_surface(base_surface, queue_handle, ());
		let toplevel = xdg_surface.get_toplevel(queue_handle, ());
		toplevel.set_title("hey, red".into());
		toplevel.set_app_id("wmantle".into());
		
		let graphics = Graphics::new(&self.display, base_surface, [320, 240]);

		base_surface.commit();
		base_surface.frame(queue_handle, ());

		self.graphics_state = Some(GraphicsState {
			surface: xdg_surface,
			toplevel,
			graphics,
		});
	}
}

impl Dispatch<WlRegistry, ()> for Window {
	fn event(
		state: &mut Self,
		registry: &WlRegistry,
		event: <WlRegistry as Proxy>::Event,
		_data: &(),
		_conn: &Connection,
		queue_handle: &QueueHandle<Self>,
	) {
		//println!("{:?}", event);
		match event {
			wl_registry::Event::Global { name, interface, version }
			if interface.as_str() == "wl_compositor" => {
				let compositor = registry.bind::<WlCompositor, _, _>(name, version, queue_handle, ());

				let surface = compositor.create_surface(queue_handle, ());
				
				let region = compositor.create_region(queue_handle, ());
				surface.set_input_region(Some(&region));

				let previous_surface = state.base_surface.replace(surface);
				assert!(previous_surface.is_none());

				if state.wm_base.is_some() && state.graphics_state.is_none() {
					state.init_xdg_surface(queue_handle);
				}
			},
			wl_registry::Event::Global { name, interface, version }
			if interface.as_str() == "xdg_wm_base" => {
				let wm_base = registry.bind::<XdgWmBase, _, _>(name, version, queue_handle, ());
				let previous_base = state.wm_base.replace(wm_base);
				assert!(previous_base.is_none());

				if state.base_surface.is_some() && state.graphics_state.is_none() {
					state.init_xdg_surface(queue_handle);
				}
			},
			_ => (),
		}
	}
}

impl Dispatch<WlCompositor, ()> for Window {
	fn event(
		_: &mut Self,
		_: &WlCompositor,
		_: wl_compositor::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		unreachable!("wl_compositor has no events")
	}
}

impl Dispatch<WlSurface, ()> for Window {
	fn event(
		_: &mut Self,
		_: &WlSurface,
		event: wl_surface::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		todo!("{:?}", event)
	}
}

impl Dispatch<XdgWmBase, ()> for Window {
	fn event(
		_: &mut Self,
		base: &XdgWmBase,
		event: xdg_wm_base::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		match event {
			xdg_wm_base::Event::Ping { serial } => {
				base.send_request(xdg_wm_base::Request::Pong { serial }).unwrap();
			},
			_ => todo!("{:?}", event)
		}
	}
}

impl Dispatch<XdgSurface, ()> for Window {
	fn event(
		_: &mut Self,
		surface: &XdgSurface,
		event: xdg_surface::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		match event {
			xdg_surface::Event::Configure { serial } => {
				println!("configure_surface: {}", serial);
				// TODO: actually configure
				surface.send_request(xdg_surface::Request::AckConfigure { serial }).unwrap()
			},
			_ => todo!("{:?}", event),
		}
	}
}

impl Dispatch<XdgToplevel, ()> for Window {
	fn event(
		state: &mut Self,
		toplevel: &XdgToplevel,
		event: xdg_toplevel::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		match event {
			xdg_toplevel::Event::ConfigureBounds { width, height } => {
				println!("max_size: {}×{}", width, height);
			},
			xdg_toplevel::Event::Configure { states, width, height }
			if width == 0 && height == 0 => {
				if !state.configured {
					// TODO: do the configuring
					state.graphics_state.as_mut().unwrap().graphics.draw(Some(Box::new([0.0; BUFFER_SIZE])));
					println!("configure");
					state.configured = true;
				} else {
					// TODO: do a reconfigure
				}
				println!("self configure: {:?}", states);
			},
			xdg_toplevel::Event::Configure { states, width, height } => {
				println!("configure: {}×{}, {:?}", width, height, states);
			},
			xdg_toplevel::Event::Close => {
				state.running = false;
			},
			_ => todo!("{:?}", event),
		}
	}
}

impl Dispatch<WlCallback, ()> for Window {
	fn event(
		state: &mut Self,
		_: &WlCallback,
		event: wl_callback::Event,
		_: &(),
		_: &Connection,
		queue_handle: &QueueHandle<Self>,
	) {
		if let wl_callback::Event::Done { callback_data } = event {
			let surface = state.base_surface.as_ref().unwrap();
			let interval = Duration::from_millis((callback_data - state.last_frame) as u64);
			state.last_frame = callback_data;
			
			surface.frame(queue_handle, ());

			let data = state.visualiser.write().unwrap()
				.fft_interval(interval);

			state.graphics_state.as_mut().unwrap().graphics.draw(data);
		} else {
			unreachable!("callback can only call done");
		}
	}
}

impl Dispatch<WlRegion, ()> for Window {
	fn event(
		_: &mut Self,
		_: &WlRegion,
		_: wl_region::Event,
		_: &(),
		_: &Connection,
		_: &QueueHandle<Self>,
	) {
		unreachable!("wl_region has no events")
	}
}
