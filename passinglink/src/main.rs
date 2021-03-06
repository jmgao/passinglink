#![no_main]
#![no_std]
#![allow(non_snake_case)]
#![feature(asm, alloc_error_handler)]

extern crate alloc;

mod allocator;

#[global_allocator]
static ALLOC: allocator::Allocator = allocator::Allocator::new();

#[alloc_error_handler]
fn alloc_error_handler(_: alloc::alloc::Layout) -> ! {
  panic!("failed to allocate");
}

extern crate panic_semihosting;

#[macro_use]
extern crate log;

#[macro_use]
extern crate proper;

#[cfg(not(feature = "no_serial"))]
use core::fmt::Write;

use cortex_m::asm::delay;
use cortex_m::interrupt;

use rtfm::app;
use rtfm::Instant;

use stm32f1xx_hal::prelude::*;

#[cfg(not(feature = "no_serial"))]
use stm32f1xx_hal::serial::{Parity, Serial, StopBits};

use stm32_usbd::{UsbBus, UsbPinsType};
use usb_device::bus;
use usb_device::prelude::*;

pub mod auth;
mod hid;

#[macro_use]
mod input;
use input::*;

#[cfg(not(feature = "no_serial"))]
mod serial;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[cfg(not(feature = "no_serial"))]
static mut SERIAL: Option<serial::BufferedSerial> = None;

static mut OUTPUT: DeviceInputs = DeviceInputs::default();

trait InfallibleInputPin {
  fn is_low(&self) -> bool;
  fn is_high(&self) -> bool;
}

impl<T: embedded_hal::digital::v2::InputPin> InfallibleInputPin for T {
  fn is_low(&self) -> bool {
    if let Ok(result) = embedded_hal::digital::v2::InputPin::is_low(self) {
      result
    } else {
      panic!("failed to read from InputPin");
    }
  }

  fn is_high(&self) -> bool {
    if let Ok(result) = embedded_hal::digital::v2::InputPin::is_high(self) {
      result
    } else {
      panic!("failed to read from InputPin");
    }
  }
}

trait InfallibleOutputPin {
  fn set_low(&mut self);
  fn set_high(&mut self);
}

impl<T: embedded_hal::digital::v2::OutputPin> InfallibleOutputPin for T {
  fn set_low(&mut self) {
    if let Err(_) = embedded_hal::digital::v2::OutputPin::set_low(self) {
      panic!("failed to write to OutputPin");
    }
  }

  fn set_high(&mut self) {
    if let Err(_) = embedded_hal::digital::v2::OutputPin::set_high(self) {
      panic!("failed to write to OutputPin");
    }
  }
}

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {
  static mut INPUT: InputPins = ();
  static mut LED: LedPins = ();

  static mut USB_DEV: UsbDevice<'static, UsbBus<UsbPinsType>> = ();
  static mut USB_HID: hid::HidClass<'static, hid::PS4Hid, UsbBus<UsbPinsType>> = ();

  #[init]
  fn init() {
    static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBus<UsbPinsType>>> = None;

    let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();

    let clocks = rcc
      .cfgr
      .use_hse(8.mhz())
      .sysclk(72.mhz())
      .pclk1(24.mhz())
      .freeze(&mut flash.acr);

    assert!(clocks.usbclk_valid());

    let mut gpioa = device.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
    let mut gpioc = device.GPIOC.split(&mut rcc.apb2);
    let mut gpiod = device.GPIOD.split(&mut rcc.apb2);

    let input = assign_inputs!(gpioa, gpiob, gpioc, gpiod);
    let mut led = assign_leds!(gpioa, gpiob, gpioc, gpiod);

    led.front.set_high();
    if let Some(ref mut r) = led.pcb_r {
      r.set_high();
    }
    if let Some(ref mut g) = led.pcb_g {
      g.set_high();
    }
    if let Some(ref mut b) = led.pcb_b {
      b.set_high();
    }

    #[cfg(not(feature = "no_serial"))]
    {
      let mut afio = device.AFIO.constrain(&mut rcc.apb2);
      let pin_tx = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
      let pin_rx = gpioa.pa3;

      let serial_config = stm32f1xx_hal::serial::Config {
        baudrate: 921_600.bps(),
        parity: Parity::ParityNone,
        stopbits: StopBits::STOP1,
      };

      let serial = Serial::usart2(
        device.USART2,
        (pin_tx, pin_rx),
        &mut afio.mapr,
        serial_config,
        clocks,
        &mut rcc.apb1,
      );

      let mut buffered_serial = serial::BufferedSerial::new(serial);

      unsafe {
        let _ = write!(buffered_serial, "\r\n\r\n");
        SERIAL = Some(buffered_serial);
        log::set_logger(SERIAL.as_ref().unwrap()).unwrap();
        log::set_max_level(log::LevelFilter::Trace);
      }
    }

    // BluePill board has a pull-up resistor on the D+ line.
    // Pull the D+ pin down to send a RESET condition to the USB bus.
    let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
    usb_dp.set_low();
    delay(clocks.sysclk().0 / 100);

    let usb_dm = gpioa.pa11;
    let usb_dp = usb_dp.into_floating_input(&mut gpioa.crh);
    *USB_BUS = Some(UsbBus::new(device.USB, (usb_dm, usb_dp)));

    let ps4_hid = hid::PS4Hid::new(unsafe { &mut OUTPUT as *mut DeviceInputs });
    let usb_hid = hid::HidClass::new(ps4_hid, USB_BUS.as_ref().unwrap());
    let usb_dev = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x1209, 0x214D))
      .manufacturer("jmgao")
      .product("Passing Link")
      .serial_number("66C623A66B214BB226X76C236B214A214CC6C236B")
      .device_class(0x00)
      .device_sub_class(0x00)
      .max_power(500)
      .max_packet_size_0(64)
      .build();

    INPUT = input;
    LED = led;
    USB_DEV = usb_dev;
    USB_HID = usb_hid;
  }

  #[task(resources = [INPUT, USB_DEV, USB_HID])]
  fn input_poll() {
    interrupt::free(|_| unsafe {
      OUTPUT.button_north.set_value(resources.INPUT.button_north.is_low());
      OUTPUT.button_east.set_value(resources.INPUT.button_east.is_low());
      OUTPUT.button_south.set_value(resources.INPUT.button_south.is_low());
      OUTPUT.button_west.set_value(resources.INPUT.button_west.is_low());

      OUTPUT.button_l1.set_value(resources.INPUT.button_l1.is_low());
      OUTPUT.button_r1.set_value(resources.INPUT.button_r1.is_low());

      let l2 = resources.INPUT.button_l2.is_low();
      OUTPUT.button_l2.set_value(l2);
      OUTPUT.axis_left_trigger.set_value(if l2 { 255 } else { 0 });

      let r2 = resources.INPUT.button_r2.is_low();
      OUTPUT.button_r2.set_value(r2);
      OUTPUT.axis_right_trigger.set_value(if r2 { 255 } else { 0 });

      OUTPUT.button_l3.set_value(resources.INPUT.button_l3.is_low());
      OUTPUT.button_r3.set_value(resources.INPUT.button_r3.is_low());

      if !resources.INPUT.mode_lock.is_low() {
        OUTPUT.button_home.set_value(resources.INPUT.button_home.is_low());
        OUTPUT.button_start.set_value(resources.INPUT.button_start.is_low());
        OUTPUT.button_select.set_value(resources.INPUT.button_select.is_low());
      }

      let _ = resources.INPUT.mode_ls.is_low();
      let _ = resources.INPUT.mode_rs.is_low();
      let _ = resources.INPUT.mode_ps3.is_low();

      OUTPUT
        .button_trackpad
        .set_value(resources.INPUT.button_trackpad.is_low());

      let (left, right) = (
        resources.INPUT.stick_left.is_low(),
        resources.INPUT.stick_right.is_low(),
      );
      let (up, down) = (resources.INPUT.stick_up.is_low(), resources.INPUT.stick_down.is_low());

      // None is neutral, Some(false) is left, Some(true) is right.
      let horizontal = match (left, right) {
        // Horizontal SOCD = neutral.
        (true, true) => None,
        (true, false) => Some(false),
        (false, true) => Some(true),
        (false, false) => None,
      };

      // None is neutral, Some(false) is down, Some(true) is up.
      let vertical = match (up, down) {
        // Vertical SOCD = up.
        (true, true) => Some(true),
        (true, false) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
      };

      if resources.INPUT.mode_ls.is_low() {
        OUTPUT.hat_dpad = Hat::Neutral;
        OUTPUT.axis_left_stick_x.set_value(match horizontal {
          Some(true) => 255,
          None => 127,
          Some(false) => 0,
        });
        OUTPUT.axis_left_stick_y.set_value(match vertical {
          Some(true) => 0,
          None => 127,
          Some(false) => 255,
        });
      } else {
        // RS is stupid, use DPad for that as well.
        OUTPUT.hat_dpad = match (horizontal, vertical) {
          (None, None) => Hat::Neutral,
          (Some(true), None) => Hat::East,
          (Some(true), Some(false)) => Hat::SouthEast,
          (None, Some(false)) => Hat::South,
          (Some(false), Some(false)) => Hat::SouthWest,
          (Some(false), None) => Hat::West,
          (Some(false), Some(true)) => Hat::NorthWest,
          (None, Some(true)) => Hat::North,
          (Some(true), Some(true)) => Hat::NorthEast,
        };
        OUTPUT.axis_left_stick_x.set_value(127);
        OUTPUT.axis_left_stick_y.set_value(127);
      }
    });

    resources.USB_HID.send();
  }

  #[task(priority = 16, schedule = [timer_tick])]
  fn timer_tick() {
    #[cfg(not(feature = "no_serial"))]
    unsafe {
      if let Some(ref mut buffered_serial) = SERIAL {
        buffered_serial.tick();
      }
    }

    schedule.timer_tick(scheduled + 72_000_000.cycles()).unwrap();
  }

  #[interrupt]
  #[cfg(not(feature = "no_serial"))]
  fn USART2() {
    unsafe {
      if let Some(ref mut serial) = SERIAL {
        serial.poll();
      }
    }
  }

  #[interrupt(resources = [USB_DEV, USB_HID])]
  fn USB_HP_CAN_TX() {
    usb_poll(&mut resources.USB_DEV, &mut resources.USB_HID);
  }

  #[interrupt(schedule = [input_poll], resources = [USB_DEV, USB_HID])]
  fn USB_LP_CAN_RX0() {
    // 900 us
    let poll_interval = (72 * 900).cycles();
    let _ = schedule.input_poll(Instant::now() + poll_interval);

    usb_poll(&mut resources.USB_DEV, &mut resources.USB_HID);
  }

  extern "C" {
    fn EXTI0();
    fn EXTI1();
  }

  #[idle(schedule = [timer_tick, input_poll])]
  fn idle() -> ! {
    schedule.timer_tick(Instant::now() + 72_000_000.cycles()).unwrap();
    schedule.input_poll(Instant::now() + 72_000.cycles()).unwrap();

    info!("passinglink v{} initialized", VERSION);

    auth::read_keypair();
    info!("ds4 keypair loaded");

    allocator::dump_state();

    auth::perform_work();
  }
};

fn usb_poll<B: bus::UsbBus>(usb_dev: &mut UsbDevice<'static, B>, hid: &mut hid::HidClass<'static, hid::PS4Hid, B>) {
  if !usb_dev.poll(&mut [hid]) {
    return;
  }
}

// Symbol used by BoringSSL functions compiled by ring.
#[no_mangle]
pub extern "C" fn __assert_func(_file: u32, _line: u32, _func: u32, _expr: u32) {
  error!("assertion triggered");
  loop {
    unsafe { asm!("nop") };
  }
}
