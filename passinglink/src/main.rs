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

use stm32f1xx_hal::gpio;
use stm32f1xx_hal::gpio::PullUp;
use stm32f1xx_hal::prelude::*;

#[cfg(not(feature = "no_serial"))]
use stm32f1xx_hal::serial::{Parity, Serial, StopBits};

use stm32_usbd::{UsbBus, UsbPinsType};
use usb_device::bus;
use usb_device::prelude::*;

pub mod auth;
mod hid;

mod input;
use input::DeviceInputs;
use input::Hat;

#[cfg(not(feature = "no_serial"))]
mod serial;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[cfg(not(feature = "no_serial"))]
static mut SERIAL: Option<serial::BufferedSerial> = None;

static mut OUTPUT: DeviceInputs = DeviceInputs::default();

pub struct InputPins {
  stick_down: gpio::gpiob::PB5<gpio::Input<PullUp>>,
  stick_up: gpio::gpiob::PB6<gpio::Input<PullUp>>,
  stick_left: gpio::gpiob::PB7<gpio::Input<PullUp>>,
  stick_right: gpio::gpiob::PB8<gpio::Input<PullUp>>,

  button_north: gpio::gpioc::PC11<gpio::Input<PullUp>>,
  button_east: gpio::gpioa::PA9<gpio::Input<PullUp>>,
  button_south: gpio::gpioa::PA10<gpio::Input<PullUp>>,
  button_west: gpio::gpioc::PC10<gpio::Input<PullUp>>,

  button_l1: gpio::gpiod::PD2<gpio::Input<PullUp>>,
  button_r1: gpio::gpioc::PC12<gpio::Input<PullUp>>,

  button_l2: gpio::gpioc::PC9<gpio::Input<PullUp>>,
  button_r2: gpio::gpioa::PA8<gpio::Input<PullUp>>,

  button_l3: gpio::gpioa::PA7<gpio::Input<PullUp>>,
  button_r3: gpio::gpiob::PB11<gpio::Input<PullUp>>,

  button_home: gpio::gpiob::PB1<gpio::Input<PullUp>>,
  button_start: gpio::gpioc::PC7<gpio::Input<PullUp>>,
  button_select: gpio::gpioc::PC8<gpio::Input<PullUp>>,

  button_trackpad: gpio::gpioa::PA6<gpio::Input<PullUp>>,
}

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {
  static mut LED: gpio::gpioc::PC13<gpio::Output<gpio::PushPull>> = ();

  static mut INPUT: InputPins = ();

  static mut USB_DEV: UsbDevice<'static, UsbBus<UsbPinsType>> = ();
  static mut USB_HID: hid::HidClass<'static, hid::PS4Hid, UsbBus<UsbPinsType>> = ();

  #[init(schedule = [timer_tick, input_poll])]
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

    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    led.set_high();

    let input = InputPins {
      stick_down: gpiob.pb5.into_pull_up_input(&mut gpiob.crl),
      stick_up: gpiob.pb6.into_pull_up_input(&mut gpiob.crl),
      stick_left: gpiob.pb7.into_pull_up_input(&mut gpiob.crl),
      stick_right: gpiob.pb8.into_pull_up_input(&mut gpiob.crh),

      button_north: gpioc.pc11.into_pull_up_input(&mut gpioc.crh),
      button_east: gpioa.pa9.into_pull_up_input(&mut gpioa.crh),
      button_south: gpioa.pa10.into_pull_up_input(&mut gpioa.crh),
      button_west: gpioc.pc10.into_pull_up_input(&mut gpioc.crh),

      button_l1: gpiod.pd2.into_pull_up_input(&mut gpiod.crl),
      button_r1: gpioc.pc12.into_pull_up_input(&mut gpioc.crh),

      button_l2: gpioc.pc9.into_pull_up_input(&mut gpioc.crh),
      button_r2: gpioa.pa8.into_pull_up_input(&mut gpioa.crh),

      button_l3: gpioa.pa7.into_pull_up_input(&mut gpioa.crl),
      button_r3: gpiob.pb11.into_pull_up_input(&mut gpiob.crh),

      button_home: gpiob.pb1.into_pull_up_input(&mut gpiob.crl),
      button_start: gpioc.pc7.into_pull_up_input(&mut gpioc.crl),
      button_select: gpioc.pc8.into_pull_up_input(&mut gpioc.crh),
      button_trackpad: gpioa.pa6.into_pull_up_input(&mut gpioa.crl),
    };

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

    info!("passinglink v{} initialized", VERSION);
    schedule.timer_tick(Instant::now() + 7_200_000.cycles()).unwrap();

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

    schedule.input_poll(Instant::now() + 72_000.cycles()).unwrap();

    LED = led;
    INPUT = input;
    USB_DEV = usb_dev;
    USB_HID = usb_hid;
  }

  #[task(priority = 1, schedule = [input_poll], resources = [INPUT, USB_DEV, USB_HID])]
  fn input_poll() {
    interrupt::free(|_| unsafe {
      OUTPUT.button_north.set_value(resources.INPUT.button_north.is_low());
      OUTPUT.button_east.set_value(resources.INPUT.button_east.is_low());
      OUTPUT.button_south.set_value(resources.INPUT.button_south.is_low());
      OUTPUT.button_west.set_value(resources.INPUT.button_west.is_low());

      OUTPUT.button_l1.set_value(resources.INPUT.button_l1.is_low());
      OUTPUT.button_r1.set_value(resources.INPUT.button_r1.is_low());

      OUTPUT.button_l2.set_value(resources.INPUT.button_l2.is_low());
      OUTPUT.button_r2.set_value(resources.INPUT.button_r2.is_low());

      OUTPUT.button_l3.set_value(resources.INPUT.button_l3.is_low());
      OUTPUT.button_r3.set_value(resources.INPUT.button_r3.is_low());

      OUTPUT.button_home.set_value(resources.INPUT.button_home.is_low());
      OUTPUT.button_start.set_value(resources.INPUT.button_start.is_low());
      OUTPUT.button_select.set_value(resources.INPUT.button_select.is_low());
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
    });

    resources.USB_HID.send();
    schedule.input_poll(scheduled + 72_000.cycles()).unwrap();
  }

  #[task(priority = 16, schedule = [timer_tick], resources = [LED])]
  fn timer_tick() {
    resources.LED.toggle();

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

  #[interrupt(resources = [USB_DEV, USB_HID])]
  fn USB_LP_CAN_RX0() {
    usb_poll(&mut resources.USB_DEV, &mut resources.USB_HID);
  }

  extern "C" {
    fn EXTI0();
    fn EXTI1();
  }

  #[idle]
  fn idle() -> ! {
    auth::read_keypair();
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
