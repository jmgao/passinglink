#![allow(unused)]

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Axis(u8);

impl Axis {
  pub fn get(self) -> u8 {
    self.0
  }

  pub fn set_value(&mut self, value: u8) {
    self.0 = value;
  }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum AxisType {
  LeftStickX,
  LeftStickY,
  RightStickX,
  RightStickY,
  LeftTrigger,
  RightTrigger,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Button(bool);

impl Button {
  pub fn get(self) -> bool {
    self.0
  }

  pub fn set_value(&mut self, value: bool) {
    self.0 = value;
  }

  pub fn set(&mut self) {
    self.set_value(true);
  }

  pub fn clear(&mut self) {
    self.set_value(false);
  }

  pub const fn default() -> Button {
    Button(false)
  }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum ButtonType {
  Start,
  Select,
  Home,
  North,
  East,
  South,
  West,
  L1,
  L2,
  L3,
  R1,
  R2,
  R3,
  Trackpad,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum Hat {
  Neutral,
  North,
  NorthEast,
  East,
  SouthEast,
  South,
  SouthWest,
  West,
  NorthWest,
}

impl Hat {
  pub const fn default() -> Hat {
    Hat::Neutral
  }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub enum HatType {
  DPad,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DeviceInputs {
  pub counter: u8,

  pub axis_left_stick_x: Axis,
  pub axis_left_stick_y: Axis,

  pub axis_right_stick_x: Axis,
  pub axis_right_stick_y: Axis,

  pub axis_left_trigger: Axis,
  pub axis_right_trigger: Axis,

  pub hat_dpad: Hat,

  /// Start/Options
  pub button_start: Button,

  /// Back/Share
  pub button_select: Button,

  /// Xbox/PS
  pub button_home: Button,

  /// Y/△
  pub button_north: Button,

  /// B/○
  pub button_east: Button,

  /// A/✖
  pub button_south: Button,

  /// X/□
  pub button_west: Button,

  pub button_l1: Button,
  pub button_l2: Button,
  pub button_l3: Button,

  pub button_r1: Button,
  pub button_r2: Button,
  pub button_r3: Button,

  pub button_trackpad: Button,
}

impl DeviceInputs {
  pub const fn default() -> DeviceInputs {
    DeviceInputs {
      counter: 0,

      axis_left_stick_x: Axis(128),
      axis_left_stick_y: Axis(128),

      axis_right_stick_x: Axis(128),
      axis_right_stick_y: Axis(128),

      axis_left_trigger: Axis(128),
      axis_right_trigger: Axis(128),

      hat_dpad: Hat::default(),
      button_start: Button::default(),
      button_select: Button::default(),
      button_home: Button::default(),

      button_north: Button::default(),
      button_east: Button::default(),
      button_south: Button::default(),
      button_west: Button::default(),

      button_l1: Button::default(),
      button_l2: Button::default(),
      button_l3: Button::default(),

      button_r1: Button::default(),
      button_r2: Button::default(),
      button_r3: Button::default(),

      button_trackpad: Button::default(),
    }
  }
}
