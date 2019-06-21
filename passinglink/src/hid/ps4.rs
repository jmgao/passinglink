use cortex_m::interrupt;

use crate::hid::{Hid, HidReportType};
use crate::input::{DeviceInputs, Hat};

#[allow(unused)]
#[repr(packed)]
struct PS4HidReport {
  report_id: u8, // Always 0x1.
  left_stick_x: u8,
  left_stick_y: u8,
  right_stick_x: u8,
  right_stick_y: u8,

  /// First 4 bits are the hat position, followed by 14 bits for buttons, followed by a 6-bit report counter.
  hat_buttons: [u8; 3],

  left_trigger: u8,
  right_trigger: u8,

  /// Trackpad and tilt, presumably.
  mystery: [u8; 54],
}

impl PS4HidReport {
  fn new() -> PS4HidReport {
    PS4HidReport {
      report_id: 0x1,
      left_stick_x: 0,
      left_stick_y: 0,
      right_stick_x: 0,
      right_stick_y: 0,
      hat_buttons: [0u8; 3],
      left_trigger: 0,
      right_trigger: 0,
      mystery: [0u8; 54],
    }
  }

  fn update(&mut self, inputs: &DeviceInputs) {
    self.left_stick_x = inputs.axis_left_stick_x.get();
    self.left_stick_y = inputs.axis_left_stick_y.get();
    self.right_stick_x = inputs.axis_right_stick_x.get();
    self.right_stick_y = inputs.axis_right_stick_y.get();

    let button_1 = inputs.button_west.get() as u8;
    let button_2 = inputs.button_south.get() as u8;
    let button_3 = inputs.button_east.get() as u8;
    let button_4 = inputs.button_north.get() as u8;
    let button_5 = inputs.button_l1.get() as u8;
    let button_6 = inputs.button_r1.get() as u8;
    let button_7 = inputs.button_l2.get() as u8;
    let button_8 = inputs.button_r2.get() as u8;
    let button_9 = inputs.button_select.get() as u8;
    let button_10 = inputs.button_start.get() as u8;
    let button_11 = inputs.button_l3.get() as u8;
    let button_12 = inputs.button_r3.get() as u8;
    let button_13 = inputs.button_home.get() as u8;
    let button_14 = inputs.button_trackpad.get() as u8;
    let hat_value = match inputs.hat_dpad {
      Hat::North => 0,
      Hat::NorthEast => 1,
      Hat::East => 2,
      Hat::SouthEast => 3,
      Hat::South => 4,
      Hat::SouthWest => 5,
      Hat::West => 6,
      Hat::NorthWest => 7,
      Hat::Neutral => 8,
    };

    self.hat_buttons = [0, 0, 0];
    self.hat_buttons[0] |= hat_value;
    self.hat_buttons[0] |= button_1 << 4 | button_2 << 5 | button_3 << 6 | button_4 << 7;
    self.hat_buttons[1] = button_5
      | button_6 << 1
      | button_7 << 2
      | button_8 << 3
      | button_9 << 4
      | button_10 << 5
      | button_11 << 6
      | button_12 << 7;
    self.hat_buttons[2] = button_13 | button_14 << 1;
    self.hat_buttons[2] |= inputs.counter & 0b00111111;

    self.left_trigger = inputs.axis_left_trigger.get();
    self.right_trigger = inputs.axis_right_trigger.get();
  }
}

struct InputWrapper(*const DeviceInputs);
unsafe impl Send for InputWrapper {}

pub struct PS4Hid {
  inputs: InputWrapper,
  report: PS4HidReport,
}

impl PS4Hid {
  pub fn new(inputs: *const DeviceInputs) -> PS4Hid {
    PS4Hid {
      inputs: InputWrapper(inputs),
      report: PS4HidReport::new(),
    }
  }
}

impl Hid for PS4Hid {
  #[rustfmt::skip]
  fn report_descriptor(&self) -> &[u8] {
    // Exact dump of the Razer Panthera's HID report descriptor.
    &[
      0x05, 0x01,        // Usage Page (Generic Desktop Ctrls)
      0x09, 0x05,        // Usage (Game Pad)
      0xA1, 0x01,        // Collection (Application)
      0x85, 0x01,        //   Report ID (1)
      0x09, 0x30,        //   Usage (X)
      0x09, 0x31,        //   Usage (Y)
      0x09, 0x32,        //   Usage (Z)
      0x09, 0x35,        //   Usage (Rz)
      0x15, 0x00,        //   Logical Minimum (0)
      0x26, 0xFF, 0x00,  //   Logical Maximum (255)
      0x75, 0x08,        //   Report Size (8)
      0x95, 0x04,        //   Report Count (4)
      0x81, 0x02,        //   Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)

      0x09, 0x39,        //   Usage (Hat switch)
      0x15, 0x00,        //   Logical Minimum (0)
      0x25, 0x07,        //   Logical Maximum (7)
      0x35, 0x00,        //   Physical Minimum (0)
      0x46, 0x3B, 0x01,  //   Physical Maximum (315)
      0x65, 0x14,        //   Unit (System: English Rotation, Length: Centimeter)
      0x75, 0x04,        //   Report Size (4)
      0x95, 0x01,        //   Report Count (1)
      0x81, 0x42,        //   Input (Data,Var,Abs,No Wrap,Linear,Preferred State,Null State)

      0x65, 0x00,        //   Unit (None)
      0x05, 0x09,        //   Usage Page (Button)
      0x19, 0x01,        //   Usage Minimum (0x01)
      0x29, 0x0E,        //   Usage Maximum (0x0E)
      0x15, 0x00,        //   Logical Minimum (0)
      0x25, 0x01,        //   Logical Maximum (1)
      0x75, 0x01,        //   Report Size (1)
      0x95, 0x0E,        //   Report Count (14)
      0x81, 0x02,        //   Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)

      0x06, 0x00, 0xFF,  //   Usage Page (Vendor Defined 0xFF00)
      0x09, 0x20,        //   Usage (0x20)
      0x75, 0x06,        //   Report Size (6)
      0x95, 0x01,        //   Report Count (1)
      0x81, 0x02,        //   Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)

      0x05, 0x01,        //   Usage Page (Generic Desktop Ctrls)
      0x09, 0x33,        //   Usage (Rx)
      0x09, 0x34,        //   Usage (Ry)
      0x15, 0x00,        //   Logical Minimum (0)
      0x26, 0xFF, 0x00,  //   Logical Maximum (255)
      0x75, 0x08,        //   Report Size (8)
      0x95, 0x02,        //   Report Count (2)
      0x81, 0x02,        //   Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)

      0x06, 0x00, 0xFF,  //   Usage Page (Vendor Defined 0xFF00)
      0x09, 0x21,        //   Usage (0x21)
      0x95, 0x36,        //   Report Count (54)
      0x81, 0x02,        //   Input (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)

      0x85, 0x05,        //   Report ID (5)
      0x09, 0x22,        //   Usage (0x22)
      0x95, 0x1F,        //   Report Count (31)
      0x91, 0x02,        //   Output (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)

      0x85, 0x03,        //   Report ID (3)
      0x0A, 0x21, 0x27,  //   Usage (0x2721)
      0x95, 0x2F,        //   Report Count (47)
      0xB1, 0x02,        //   Feature (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
      0xC0,              // End Collection


      0x06, 0xF0, 0xFF,  // Usage Page (Vendor Defined 0xFFF0)
      0x09, 0x40,        // Usage (0x40)
      0xA1, 0x01,        // Collection (Application)
      0x85, 0xF0,        //   Report ID (-16)
      0x09, 0x47,        //   Usage (0x47)
      0x95, 0x3F,        //   Report Count (63)
      0xB1, 0x02,        //   Feature (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
      0x85, 0xF1,        //   Report ID (-15)
      0x09, 0x48,        //   Usage (0x48)
      0x95, 0x3F,        //   Report Count (63)
      0xB1, 0x02,        //   Feature (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
      0x85, 0xF2,        //   Report ID (-14)
      0x09, 0x49,        //   Usage (0x49)
      0x95, 0x0F,        //   Report Count (15)
      0xB1, 0x02,        //   Feature (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
      0x85, 0xF3,        //   Report ID (-13)
      0x0A, 0x01, 0x47,  //   Usage (0x4701)
      0x95, 0x07,        //   Report Count (7)
      0xB1, 0x02,        //   Feature (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
      0xC0,              // End Collection
    ]
  }

  fn set_report(&mut self, report_type: HidReportType, report_id: u8, data: &[u8]) -> Result<(), ()> {
    info!(
      "PS4Device::set_report({:?}, {:#x}, {} bytes) = {:x?}",
      report_type,
      report_id,
      data.len(),
      data
    );
    Ok(())
  }

  fn get_report(&mut self, report_type: HidReportType, report_id: u8, length: Option<u16>) -> Result<&[u8], ()> {
    if let Some(len) = length {
      info!(
        "PS4Hid::get_report({:?}, {:#x}): expecting {} bytes",
        report_type, report_id, len
      );
    }

    if report_id == 0 {
      interrupt::free(|_| {
        let inputs = unsafe { &*(self.inputs.0) };
        self.report.update(inputs);
      });

      let slice = unsafe {
        core::slice::from_raw_parts(
          (&self.report) as *const PS4HidReport as *const u8,
          core::mem::size_of_val(&self.report),
        )
      };
      Ok(slice)
    } else if report_id == 0x3 {
      // Unclear, copied from an actual device..
      if length == Some(48) {
        Ok(&[
          0x3, 0x21, 0x27, 0x4, 0x40, 0x7, 0x2c, 0x56, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0xd, 0xd, 0x0,
          0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
          0x0, 0x0, 0x0, 0x0, 0x0,
        ])
      } else {
        error!("unexpected length for report 0x3, expected 48, got {}", length.unwrap());
        Err(())
      }
    } else {
      error!("unexpected report id: {:#x}", report_id);
      Err(())
    }
  }
}
