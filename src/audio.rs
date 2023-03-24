use std::sync::{Arc, RwLock};

use pipewire::{stream::*, properties, spa::{Direction, pod::{deserialize::PodDeserializer, Value}, utils::Id}, MainLoop};

use crate::{audio::pod_choice_default::Fixate, visualiser::BufferManager};

mod spa_audio_info_raw;
mod pod_choice_default;

#[derive(Debug)]
struct StreamConfiguration {
	rate: u32,
	channels: u32,
	format: u32,
}

#[derive(Default)]
struct StreamData {
	configuration: Option<StreamConfiguration>,
	visualiser: Arc<RwLock<BufferManager>>,
}

pub(crate) fn main(
	visualiser: Arc<RwLock<BufferManager>>,
) {
	std::thread::spawn(move || {
		let mainloop = MainLoop::new().unwrap();
		let _stream = stream(&mainloop, visualiser);
		mainloop.run();
	});
}

fn stream(
	mainloop: &MainLoop,
	visualiser: Arc<RwLock<BufferManager>>,
) -> Stream<StreamData> {
	let stream = Stream::<StreamData>::with_user_data(
		mainloop,
		"audio-capture",
		properties! {
			*pipewire::keys::NODE_NAME => env!("CARGO_PKG_NAME"),
			*pipewire::keys::MEDIA_TYPE => "Audio",
			*pipewire::keys::MEDIA_CATEGORY => "Capture",
			*pipewire::keys::STREAM_CAPTURE_SINK => "true",
		},
		StreamData {
			configuration: None,
			visualiser,
		},
	)
	.param_changed(|id, data, raw_pod| {
		if id == libspa_sys::SPA_PARAM_Format {
			let pointer = std::ptr::NonNull::new(raw_pod.cast_mut()).unwrap();
			let object = unsafe {
				PodDeserializer::deserialize_ptr::<Value>(pointer).unwrap()
			};

			if let Value::Object(object) = object {
				data.configuration = None;

				let media_type: Id = object.properties.iter()
					.find(|p| p.key == libspa_sys::SPA_FORMAT_mediaType)
					.unwrap().value
					.fixate().unwrap();
				
				let media_subtype: Id = object.properties.iter()
					.find(|p| p.key == libspa_sys::SPA_FORMAT_mediaSubtype)
					.unwrap().value
					.fixate().unwrap();
				
				let format: Id = object.properties.iter()
					.find(|p| p.key == libspa_sys::SPA_FORMAT_AUDIO_format)
					.unwrap().value
					.fixate().unwrap();
				
				let rate: i32 = object.properties.iter()
					.find(|p| p.key == libspa_sys::SPA_FORMAT_AUDIO_rate)
					.unwrap().value
					.fixate().unwrap();
				
				let channels: i32 = object.properties.iter()
					.find(|p| p.key == libspa_sys::SPA_FORMAT_AUDIO_channels)
					.unwrap().value
					.fixate().unwrap();
				
				let is_audio = media_type.0 == libspa_sys::SPA_MEDIA_TYPE_audio;
				let is_raw = media_subtype.0 == libspa_sys::SPA_MEDIA_SUBTYPE_raw;
				if is_audio && is_raw {
					data.configuration = Some(StreamConfiguration {
						rate: rate as u32,
						channels: channels as u32,
						format: format.0,
					});
				}
			}
		}
	})
	.process(|stream, StreamData { configuration, visualiser }| {
		if let Some(mut buffer) = stream.dequeue_buffer() {
			// TODO: this is just the left channel — maybe handle all channels
			let channel = buffer.datas_mut().get_mut(0).unwrap();
			let offset = channel.chunk().offset();
			let chunk = channel.chunk(); 
			let size = chunk.size() as usize;
			let stride = chunk.stride();
			let data = channel.data(); 

			if let Some(data) = data {
				let rate = configuration.as_ref().unwrap().rate;

				let cast_buffer: &[f32] = unsafe {
					std::slice::from_raw_parts(data.as_ptr().cast(), size / std::mem::size_of::<f32>())
				};
				visualiser.write().unwrap()
					.fill_buffer(cast_buffer, rate)
			}
		}
	})
	.create().unwrap();

	let params = spa_audio_info_raw::SpaAudioInfoRaw::empty().as_pod().unwrap();

	stream.connect(
		Direction::Input,
		None,
		StreamFlags::AUTOCONNECT | StreamFlags::RT_PROCESS | StreamFlags::MAP_BUFFERS,
		&mut [params.as_ptr().cast()],
	).unwrap();
	
	stream
}
