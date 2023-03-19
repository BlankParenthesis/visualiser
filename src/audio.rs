use std::sync::{Arc, RwLock};

use pipewire::{stream::*, properties, spa::{Direction, pod::{deserialize::PodDeserializer, Value}, utils::Id}, MainLoop};

use crate::{audio::pod_choice_default::Fixate, visualiser::BufferManager};

mod spa_audio_info_raw;
//mod pod_object_param_props;
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
			*pipewire::keys::NODE_NAME => "WMantle",
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
		//unsafe {
		//println!("{{ id: {}, size: {}, type: {} }}", id, (*raw_pod).size, (*raw_pod).type_);
		//}

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
				
				//println!("media_type: {:?}", media_type);
				//println!("media_subtype: {:?}", media_subtype);
				//println!("format: {:?}", format);
				//println!("rate: {:?}", rate);
				//println!("channels: {:?}", channels);

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
	.state_changed(move |old, new| {
		println!("{:?} → {:?}", old, new);
	})
	.process(|stream, StreamData { configuration, visualiser }| {
		if let Some(mut buffer) = stream.dequeue_buffer() {
			// set this from wayland draw callback
			// (we'll be one frame behind, but it saves doing fft every time we get audio data)
			// that said, what if this function is called less than once per frame?
			// split each buffer into smaller parts I guess?
			// could be difficult, hmm…
			let needs_update = true;

			let channel = buffer.datas_mut().get_mut(1).unwrap();
			let offset = channel.chunk().offset();
			let chunk = channel.chunk(); 
			let size = chunk.size() as usize;
			let stride = chunk.stride();
			let data = channel.data(); 

			if needs_update {
				if let Some(data) = data {
					//std::io::Write::write_all(&mut std::io::stdout(), &[0x1b, b'[', 1, b'A']).unwrap();
					//println!("process {:?} ({} / {}) +{} chunks", size, data.len(), stride, offset);

					let rate = configuration.as_ref().unwrap().rate;

					let cast_buffer: &[f32] = unsafe {
						std::slice::from_raw_parts(data.as_ptr().cast(), size / std::mem::size_of::<f32>())
					};
					visualiser.write().unwrap()
						.fill_buffer(cast_buffer, rate)
				}
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

/*
	//if matches!(new, StreamState::Paused) {
	//	let a = dequeue_cell.borrow();
	//	let buf = unsafe { a.as_ref().unwrap().dequeue_raw_buffer() };
	//	println!("buf {:?}", buf);
	//}

	//.param_changed(|id, (), raw_pod| {
	//	let pointer = std::ptr::NonNull::new(raw_pod.cast_mut()).unwrap();
	//	let object = unsafe {
	//		PodDeserializer::deserialize_ptr::<Value>(pointer).unwrap()
	//	};

	//	if let Value::Object(object) = object {
	//		//assert!(object.type_ == libspa_sys::SPA_TYPE_OBJECT_Props);

	//		let media_type = object.properties.iter().find(|p| p.key == libspa_sys::SPA_FORMAT_mediaType);
	//		let media_subtype = object.properties.iter().find(|p| p.key == libspa_sys::SPA_FORMAT_mediaSubtype);

	//		//println!("{}: {:?}", id, object);
	//		println!("{}, media_type: {:?}, media_subtype: {:?}", object.type_, media_type, media_subtype);

	//	} else {
	//		println!("owo");
	//	}
	//})
	.control_info(|_, info| {
		//println!("help: {:?}", unsafe { *info });
	})
	.drained(|| {
		println!("drained");
	})
	.add_buffer(|buffer| {
		println!("+buffer: {:?}", buffer);
	})
	.remove_buffer(|buffer| {
		println!("-buffer: {:?}", buffer);
	})
*/