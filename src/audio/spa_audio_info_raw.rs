use std::io::{Write, Seek, Cursor};

use pipewire::{spa::{pod::{serialize::*, PropertyFlags}, utils::Id}};

type ChannelPosition = libspa_sys::spa_audio_channel;

// TODO: enums for format and ChannelPosition

pub(crate) struct SpaAudioInfoRaw {
	pub format: libspa_sys::spa_audio_format,
	pub flags: u32,
	pub rate: u32,
	pub channels: Vec<Option<ChannelPosition>>,
}

impl SpaAudioInfoRaw {
	pub fn empty() -> Self {
		Self {
			format: libspa_sys::SPA_AUDIO_FORMAT_UNKNOWN,
			flags: 0,
			rate: 0,
			channels: vec![]
		}
	}
}

impl SpaAudioInfoRaw {
	pub fn as_pod(&self) -> Result<Box<[u8]>, GenError> {
		let mut pod = Vec::<u8>::new();
		let cursor = Cursor::new(&mut pod);
	
		PodSerializer::serialize(cursor, self)?;

		Ok(pod.into_boxed_slice())
	}
}

impl PodSerialize for SpaAudioInfoRaw {
	fn serialize<O: Write + Seek>(
		&self,
		serializer: PodSerializer<O>,
	) -> Result<SerializeSuccess<O>, GenError> {
		let mut object_serializer = serializer.serialize_object(
			libspa_sys::SPA_TYPE_OBJECT_Format,
			libspa_sys::SPA_PARAM_EnumFormat,
		)?;
		object_serializer.serialize_property(
			libspa_sys::SPA_FORMAT_mediaType,
			&Id(libspa_sys::SPA_MEDIA_TYPE_audio),
			PropertyFlags::READONLY,
		)?;
		object_serializer.serialize_property(
			libspa_sys::SPA_FORMAT_mediaSubtype,
			&Id(libspa_sys::SPA_MEDIA_SUBTYPE_raw),
			PropertyFlags::READONLY,
		)?;
		if self.format != libspa_sys::SPA_AUDIO_FORMAT_UNKNOWN {
			object_serializer.serialize_property(
				libspa_sys::SPA_FORMAT_AUDIO_format,
				&Id(self.format),
				PropertyFlags::READONLY,
			)?;
		}
		if self.rate != 0 {
			object_serializer.serialize_property(
				libspa_sys::SPA_FORMAT_AUDIO_rate,
				&Id(self.rate),
				PropertyFlags::READONLY,
			)?;
		}
		if !self.channels.is_empty() {
			object_serializer.serialize_property(
				libspa_sys::SPA_FORMAT_AUDIO_channels,
				&Id(self.channels.len() as u32),
				PropertyFlags::READONLY,
			)?;

			if self.flags & libspa_sys::SPA_AUDIO_FLAG_UNPOSITIONED == 0 {
				let channels = self.channels.iter()
					.map(|c| match c {
						Some(id) => Id(*id),
						None => Id(0),
					})
					.collect::<Vec<_>>();

				object_serializer.serialize_property(
					libspa_sys::SPA_FORMAT_AUDIO_position,
					channels.as_slice(),
					PropertyFlags::READONLY,
				)?;
			}
		}
		object_serializer.end()
	}
}