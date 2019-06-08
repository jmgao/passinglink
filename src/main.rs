#![no_main]
#![no_std]
#![allow(non_snake_case)]
#![feature(asm)]

extern crate panic_semihosting;

use core::fmt::Write;

use rtfm::app;
use rtfm::Instant;
use stm32f1xx_hal::prelude::*;

use stm32f1xx_hal::gpio;
use stm32f1xx_hal::serial::{Parity, Serial, StopBits};

use stm32f103xx_usb::UsbBus;
use usb_device::bus;
use usb_device::prelude::*;

mod serial;
use serial::BufferedSerial;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {
  static mut LED: gpio::gpioc::PC13<gpio::Output<gpio::PushPull>> = ();
  static mut SERIAL: BufferedSerial = ();

  static mut USB_DEV: UsbDevice<'static, UsbBus> = ();

  #[init(schedule = [periodic])]
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

    let mut serial = BufferedSerial::new(Serial::usart2(
      device.USART2,
      (pin_tx, pin_rx),
      &mut afio.mapr,
      serial_config,
      clocks,
      &mut rcc.apb1,
    ));

    *USB_BUS = Some(UsbBus::usb_with_reset(
      device.USB,
      &mut rcc.apb1,
      &clocks,
      &mut gpioa.crh,
      gpioa.pa12,
    ));

    let mut usb_dev = UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x1209, 0xD66C))
      .manufacturer("jmgao")
      .product("Passing Link")
      .serial_number("66C623A66B214BB226X76C236B214A214CC6C236B")
      .device_class(0x00)
      .device_sub_class(0x00)
      .max_power(500)
      .build();

    usb_dev.force_reset().expect("reset failed");

    write!(serial, "passinglink v{} initialized\r\n", VERSION).unwrap();
    schedule.periodic(Instant::now() + 7_200_000.cycles()).unwrap();

    LED = led;
    SERIAL = serial;
    USB_DEV = usb_dev;
  }

  #[task(schedule = [periodic], resources = [LED, SERIAL])]
  fn periodic() {
    resources.LED.toggle();
    schedule.periodic(scheduled + 7_200_000.cycles()).unwrap();
  }

  #[interrupt(resources = [SERIAL])]
  fn USART2() {
    resources.SERIAL.poll();
  }

  #[interrupt(resources = [USB_DEV])]
  fn USB_HP_CAN_TX() {
    usb_poll(&mut resources.USB_DEV);
  }

  #[interrupt(resources = [USB_DEV])]
  fn USB_LP_CAN_RX0() {
    usb_poll(&mut resources.USB_DEV);
  }

  extern "C" {
    fn EXTI0();
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

fn usb_poll<B: bus::UsbBus>(usb_dev: &mut UsbDevice<'static, B>) {
  if !usb_dev.poll(&mut []) {
    return;
  }
}
