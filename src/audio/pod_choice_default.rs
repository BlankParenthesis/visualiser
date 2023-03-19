use libspa_sys::{spa_rectangle as Rectangle, spa_fraction as Fraction};
use pipewire::{spa::{pod::{ChoiceValue, CanonicalFixedSizedPod, Value}, utils::{Choice, ChoiceEnum, Id, Fd}}};

pub trait Fixate<T> {
	fn fixate(&self) -> Result<T, ()>;
}

impl Fixate<i32> for Value {
	fn fixate(&self) -> Result<i32, ()> {
		match self {
			Value::Int(int) => Ok(*int),
			Value::Choice(choice) => choice.choice_default(),
			_ => Err(()),
		}
	}
}

impl Fixate<Id> for Value {
	fn fixate(&self) -> Result<Id, ()> {
		match self {
			Value::Id(id) => Ok(*id),
			Value::Choice(choice) => choice.choice_default(),
			_ => Err(()),
		}
	}
}

impl Fixate<f32> for Value {
	fn fixate(&self) -> Result<f32, ()> {
		match self {
			Value::Float(id) => Ok(*id),
			Value::Choice(choice) => choice.choice_default(),
			_ => Err(()),
		}
	}
}

pub trait ChoiceDefault<T> {
	fn choice_default(&self) -> Result<T, ()>;
}

impl<T: CanonicalFixedSizedPod + Copy> ChoiceDefault<T> for Choice<T> {
	fn choice_default(&self) -> Result<T, ()> {
		Ok(match &self.1 {
			ChoiceEnum::None(value) => *value,
			ChoiceEnum::Range { default, .. } => *default,
			ChoiceEnum::Step { default, .. } => *default,
			ChoiceEnum::Enum { default, .. } => *default,
			ChoiceEnum::Flags { default, .. } => *default,
		})
	}
}

impl ChoiceDefault<i32> for ChoiceValue {
	fn choice_default(&self) -> Result<i32, ()> {
		if let ChoiceValue::Int(choice) = self {
			choice.choice_default()
		} else {
			Err(())
		}
	}
}

impl ChoiceDefault<i64> for ChoiceValue {
	fn choice_default(&self) -> Result<i64, ()> {
		if let ChoiceValue::Long(choice) = self {
			choice.choice_default()
		} else {
			Err(())
		}
	}
}

impl ChoiceDefault<f32> for ChoiceValue {
	fn choice_default(&self) -> Result<f32, ()> {
		if let ChoiceValue::Float(choice) = self {
			choice.choice_default()
		} else {
			Err(())
		}
	}
}

impl ChoiceDefault<f64> for ChoiceValue {
	fn choice_default(&self) -> Result<f64, ()> {
		if let ChoiceValue::Double(choice) = self {
			choice.choice_default()
		} else {
			Err(())
		}
	}
}

impl ChoiceDefault<Id> for ChoiceValue {
	fn choice_default(&self) -> Result<Id, ()> {
		if let ChoiceValue::Id(choice) = self {
			choice.choice_default()
		} else {
			Err(())
		}
	}
}

impl ChoiceDefault<Rectangle> for ChoiceValue {
	fn choice_default(&self) -> Result<Rectangle, ()> {
		if let ChoiceValue::Rectangle(choice) = self {
			choice.choice_default()
		} else {
			Err(())
		}
	}
}

impl ChoiceDefault<Fraction> for ChoiceValue {
	fn choice_default(&self) -> Result<Fraction, ()> {
		if let ChoiceValue::Fraction(choice) = self {
			choice.choice_default()
		} else {
			Err(())
		}
	}
}
impl ChoiceDefault<Fd> for ChoiceValue {
	fn choice_default(&self) -> Result<Fd, ()> {
		if let ChoiceValue::Fd(choice) = self {
			choice.choice_default()
		} else {
			Err(())
		}
	}
}