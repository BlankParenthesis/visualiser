use pipewire::spa::pod::{deserialize::*, Value, PropertyFlags, ValueArray};

#[derive(Debug)]
pub(crate) struct ObjectParamProps {
	volume: f32,
	volume_flags: PropertyFlags,
	mute: bool,
	mute_flags: PropertyFlags,
	channel_volumes: Box<[f32]>,
	channel_volumes_flags: PropertyFlags,
}

impl Default for ObjectParamProps {
	fn default() -> Self {
		Self {
			volume: 0.0,
			volume_flags: PropertyFlags::empty(),
			mute: false,
			mute_flags: PropertyFlags::empty(),
			channel_volumes: vec![].into_boxed_slice(),
			channel_volumes_flags: PropertyFlags::empty(),
		}
	}
}

impl ObjectParamProps {
	pub unsafe fn from_pod<'de>(
		raw_pod: *const libspa_sys::spa_pod
	) -> Result<Self, DeserializeError<&'de [u8]>> {
		let pointer = std::ptr::NonNull::new(raw_pod.cast_mut()).unwrap();
		PodDeserializer::deserialize_ptr(pointer)

	}
}

struct 

struct ObjectParamPropsVisitor;

impl<'de> Visitor<'de> for ObjectParamPropsVisitor {
	type Value = ObjectParamProps;
	type ArrayElem = ();

	fn visit_object(
		&self,
		deserializer: &mut ObjectPodDeserializer<'de>,
	) -> Result<Self::Value, DeserializeError<&'de [u8]>> {
		let mut props = ObjectParamProps::default();

		let volume = deserializer
			.deserialize_property_key(libspa_sys::SPA_PROP_volume)?;
		if let (Value::Float(volume), flags) = volume {
			props.volume = volume;
			props.volume_flags = flags;
		} else {
			return Err(DeserializeError::InvalidType);
		}

		let mute = deserializer
			.deserialize_property_key(libspa_sys::SPA_PROP_mute)?;
		if let (Value::Bool(mute), flags) = mute {
			props.mute = mute;
			props.mute_flags = flags;
		} else {
			return Err(DeserializeError::InvalidType);
		}

		let channel_volumes = deserializer
			.deserialize_property_key(libspa_sys::SPA_PROP_channelVolumes)?;
		if let (ValueArray::Float(volumes), flags) = channel_volumes {
			props.channel_volumes = volumes.into_boxed_slice();
			props.channel_volumes_flags = flags;
		} else {
			return Err(DeserializeError::InvalidType);
		}

		println!("{:?}", props);

		Ok(props)
	}
}

impl<'de> PodDeserialize<'de> for ObjectParamProps {
	fn deserialize(
		deserializer: PodDeserializer<'de>,
	) -> Result<(Self, DeserializeSuccess<'de>), DeserializeError<&'de [u8]>> {
		deserializer.deserialize_object(ObjectParamPropsVisitor)
	}
}