#![no_main]
#![no_std]
#![allow(non_snake_case)]
#![feature(asm)]

extern crate panic_semihosting;

#[macro_use]
extern crate log;

#[macro_use]
extern crate proper;

use core::fmt::Write;

use cortex_m::interrupt;

use rtfm::app;
use rtfm::Instant;

use stm32f1xx_hal::gpio;
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::serial::{Parity, Serial, StopBits};

use stm32f103xx_usb::UsbBus;
use usb_device::bus;
use usb_device::prelude::*;

mod hid;

mod input;
use input::DeviceInputs;
use input::Hat;

mod serial;
use serial::BufferedSerial;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

static mut SERIAL: Option<BufferedSerial> = None;
static mut INPUTS: DeviceInputs = DeviceInputs::default();

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {
  static mut LED: gpio::gpioc::PC13<gpio::Output<gpio::PushPull>> = ();

  static mut USB_DEV: UsbDevice<'static, UsbBus> = ();
  static mut USB_HID: hid::HidClass<'static, hid::PS4Hid, UsbBus> = ();

  #[init(schedule = [timer_tick, input_poll])]
  fn init() {
    static mut USB_BUS: Option<bus::UsbBusAllocator<UsbBus>> = None;

    let mut flash = device.FLASH.constrain();
    let mut rcc = device.RCC.constrain();

    let clocks = rcc
      .cfgr
      .use_hse(8.mhz())
      .sysclk(72.mhz())
      .pclk1(24.mhz())
      .freeze(&mut flash.acr);

    assert!(clocks.usbclk_valid());

    let mut gpioc = device.GPIOC.split(&mut rcc.apb2);
    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    led.set_high();

    let mut afio = device.AFIO.constrain(&mut rcc.apb2);
    let mut gpioa = device.GPIOA.split(&mut rcc.apb2);

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

    let mut buffered_serial = BufferedSerial::new(serial);

    unsafe {
      let _ = write!(buffered_serial, "\r\n\r\n");
      SERIAL = Some(buffered_serial);
      log::set_logger(SERIAL.as_ref().unwrap()).unwrap();
      log::set_max_level(log::LevelFilter::Trace);
    }
    info!("passinglink v{} initialized", VERSION);
    schedule.timer_tick(Instant::now() + 7_200_000.cycles()).unwrap();

    *USB_BUS = Some(UsbBus::usb_with_reset(
      device.USB,
      &mut rcc.apb1,
      &clocks,
      &mut gpioa.crh,
      gpioa.pa12,
    ));

    let ps4_hid = hid::PS4Hid::new(unsafe { &mut INPUTS as *mut DeviceInputs });
    let usb_hid = hid::HidClass::new(ps4_hid, USB_BUS.as_ref().unwrap());
    let mut usb_dev = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x1209, 0x214D))
      .manufacturer("jmgao")
      .product("Passing Link")
      .serial_number("66C623A66B214BB226X76C236B214A214CC6C236B")
      .device_class(0x00)
      .device_sub_class(0x00)
      .max_power(500)
      .max_packet_size_0(64)
      .build();

    usb_dev.force_reset().expect("reset failed");

    schedule.input_poll(Instant::now() + 72_000.cycles()).unwrap();

    LED = led;
    USB_DEV = usb_dev;
    USB_HID = usb_hid;
  }

  #[task(priority = 1, schedule = [input_poll], resources = [USB_DEV, USB_HID])]
  fn input_poll() {
    interrupt::free(|_| unsafe {
      static mut COUNTER: u32 = 0;
      COUNTER = COUNTER.wrapping_add(1);

      // Press home once after 1 second, then 2 seconds later X, and then start doing SPDs every 17 milliseconds after.
      if COUNTER == 1000 {
        INPUTS.button_home.set();
      } else if COUNTER == 1250 {
        INPUTS.button_home.clear();
      } else if COUNTER == 3000 {
        INPUTS.button_south.set();
      } else if COUNTER == 3250 {
        INPUTS.button_south.clear();
      } else if COUNTER == 4000 {
        INPUTS.button_east.set();
      } else if COUNTER == 4250 {
        INPUTS.button_east.clear();
      } else if COUNTER > 5000 {
        let input = COUNTER / 17;
        INPUTS.hat_dpad = match input % 8 {
          0 => Hat::North,
          1 => Hat::NorthEast,
          2 => Hat::East,
          3 => Hat::SouthEast,
          4 => Hat::South,
          5 => Hat::SouthWest,
          6 => Hat::West,
          7 => Hat::NorthWest,
          _ => unreachable!(),
        };
      }
    });

    resources.USB_HID.send();
    schedule.input_poll(scheduled + 72_000.cycles()).unwrap();
  }

  #[task(priority = 16, schedule = [timer_tick], resources = [LED])]
  fn timer_tick() {
    resources.LED.toggle();

    unsafe {
      if let Some(ref mut buffered_serial) = SERIAL {
        buffered_serial.tick();
      }
    }

    schedule.timer_tick(scheduled + 72_000_000.cycles()).unwrap();
  }

  #[interrupt]
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
    loop {
      unsafe {
        asm!("nop");
      }
    }
  }
};

fn usb_poll<B: bus::UsbBus>(usb_dev: &mut UsbDevice<'static, B>, hid: &mut hid::HidClass<'static, hid::PS4Hid, B>) {
  if !usb_dev.poll(&mut [hid]) {
    return;
  }
}
