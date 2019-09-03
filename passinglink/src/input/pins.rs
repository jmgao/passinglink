// 0.3 and before
#[cfg(feature = "0.3")]
#[macro_use]
mod detail {
  use stm32f1xx_hal::gpio::gpioa::*;
  use stm32f1xx_hal::gpio::gpiob::*;
  use stm32f1xx_hal::gpio::gpioc::*;
  use stm32f1xx_hal::gpio::gpiod::*;
  use stm32f1xx_hal::gpio::{Input, Output, PullUp, PushPull};

  pub struct InputPins {
    pub stick_down: PB5<Input<PullUp>>,
    pub stick_up: PB6<Input<PullUp>>,
    pub stick_left: PB7<Input<PullUp>>,
    pub stick_right: PB8<Input<PullUp>>,

    pub button_north: PC11<Input<PullUp>>,
    pub button_east: PA9<Input<PullUp>>,
    pub button_south: PA10<Input<PullUp>>,
    pub button_west: PC10<Input<PullUp>>,

    pub button_l1: PD2<Input<PullUp>>,
    pub button_r1: PC12<Input<PullUp>>,

    pub button_l2: PC9<Input<PullUp>>,
    pub button_r2: PA8<Input<PullUp>>,

    pub button_l3: PA7<Input<PullUp>>,
    pub button_r3: PB11<Input<PullUp>>,

    pub button_home: PB1<Input<PullUp>>,
    pub button_start: PC7<Input<PullUp>>,
    pub button_select: PC8<Input<PullUp>>,

    pub button_trackpad: PA6<Input<PullUp>>,

    pub mode_lock: PB0<Input<PullUp>>,
    pub mode_ls: PC5<Input<PullUp>>,
    pub mode_rs: PC4<Input<PullUp>>,
    pub mode_ps3: PB10<Input<PullUp>>,
  }

  macro_rules! assign_inputs {
    ($gpioa: expr, $gpiob: expr, $gpioc: expr, $gpiod: expr) => {{
      let (_a, _b, _c, _d) = (&mut $gpioa, &mut $gpiob, &mut $gpioc, &mut $gpiod);
      InputPins {
        stick_down: $gpiob.pb5.into_pull_up_input(&mut $gpiob.crl),
        stick_up: $gpiob.pb6.into_pull_up_input(&mut $gpiob.crl),
        stick_left: $gpiob.pb7.into_pull_up_input(&mut $gpiob.crl),
        stick_right: $gpiob.pb8.into_pull_up_input(&mut $gpiob.crh),

        button_north: $gpioc.pc11.into_pull_up_input(&mut $gpioc.crh),
        button_east: $gpioa.pa9.into_pull_up_input(&mut $gpioa.crh),
        button_south: $gpioa.pa10.into_pull_up_input(&mut $gpioa.crh),
        button_west: $gpioc.pc10.into_pull_up_input(&mut $gpioc.crh),

        button_l1: $gpiod.pd2.into_pull_up_input(&mut $gpiod.crl),
        button_r1: $gpioc.pc12.into_pull_up_input(&mut $gpioc.crh),

        button_l2: $gpioc.pc9.into_pull_up_input(&mut $gpioc.crh),
        button_r2: $gpioa.pa8.into_pull_up_input(&mut $gpioa.crh),

        button_l3: $gpioa.pa7.into_pull_up_input(&mut $gpioa.crl),
        button_r3: $gpiob.pb11.into_pull_up_input(&mut $gpiob.crh),

        button_home: $gpiob.pb1.into_pull_up_input(&mut $gpiob.crl),
        button_start: $gpioc.pc7.into_pull_up_input(&mut $gpioc.crl),
        button_select: $gpioc.pc8.into_pull_up_input(&mut $gpioc.crh),
        button_trackpad: $gpioa.pa6.into_pull_up_input(&mut $gpioa.crl),

        mode_lock: $gpiob.pb0.into_pull_up_input(&mut $gpiob.crl),
        mode_ls: $gpioc.pc5.into_pull_up_input(&mut $gpioc.crl),
        mode_rs: $gpioc.pc4.into_pull_up_input(&mut $gpioc.crl),
        mode_ps3: $gpiob.pb10.into_pull_up_input(&mut $gpiob.crh),
      }
    }};
  }

  pub struct LedPins {
    pub front: PC6<Output<PushPull>>,
    pub pcb_r: Option<PCx<Output<PushPull>>>,
    pub pcb_g: Option<PCx<Output<PushPull>>>,
    pub pcb_b: Option<PCx<Output<PushPull>>>,
  }

  macro_rules! assign_leds {
    ($gpioa: expr, $gpiob: expr, $gpioc: expr, $gpiod: expr) => {{
      LedPins {
        front: $gpioc.pc6.into_push_pull_output(&mut $gpioc.crl),
        pcb_r: None,
        pcb_g: None,
        pcb_b: None,
      }
    }}
  }
}

// 0.4.
#[cfg(all(not(feature = "0.3"), not(feature="bluepill")))]
#[macro_use]
mod detail {
  use stm32f1xx_hal::gpio::gpioa::*;
  use stm32f1xx_hal::gpio::gpiob::*;
  use stm32f1xx_hal::gpio::gpioc::*;
  use stm32f1xx_hal::gpio::gpiod::*;
  use stm32f1xx_hal::gpio::{Input, Output, PullUp, PushPull};

  pub struct InputPins {
    pub stick_down: PA10<Input<PullUp>>,
    pub stick_up: PA9<Input<PullUp>>,
    pub stick_left: PA8<Input<PullUp>>,
    pub stick_right: PC9<Input<PullUp>>,

    pub button_north: PC6<Input<PullUp>>,
    pub button_east: PB13<Input<PullUp>>,
    pub button_south: PB14<Input<PullUp>>,
    pub button_west: PB15<Input<PullUp>>,

    pub button_l1: PC8<Input<PullUp>>,
    pub button_r1: PC7<Input<PullUp>>,

    pub button_l2: PB11<Input<PullUp>>,
    pub button_r2: PB12<Input<PullUp>>,

    pub button_l3: PC12<Input<PullUp>>,
    pub button_r3: PD2<Input<PullUp>>,

    pub button_home: PC15<Input<PullUp>>,
    pub button_start: PC4<Input<PullUp>>,
    pub button_select: PB10<Input<PullUp>>,
    pub button_trackpad: PB5<Input<PullUp>>,

    pub mode_lock: PC14<Input<PullUp>>,
    pub mode_ls: PC13<Input<PullUp>>,
    pub mode_rs: PC10<Input<PullUp>>,
    pub mode_ps3: PC11<Input<PullUp>>,
  }

  macro_rules! assign_inputs {
    ($gpioa: expr, $gpiob: expr, $gpioc: expr, $gpiod: expr) => {{
      let (_a, _b, _c, _d) = (&mut $gpioa, &mut $gpiob, &mut $gpioc, &mut $gpiod);
      InputPins {
        stick_down: $gpioa.pa10.into_pull_up_input(&mut $gpioa.crh),
        stick_up: $gpioa.pa9.into_pull_up_input(&mut $gpioa.crh),
        stick_left: $gpioa.pa8.into_pull_up_input(&mut $gpioa.crh),
        stick_right: $gpioc.pc9.into_pull_up_input(&mut $gpioc.crh),

        button_north: $gpioc.pc6.into_pull_up_input(&mut $gpioc.crl),
        button_east: $gpiob.pb13.into_pull_up_input(&mut $gpiob.crh),
        button_south: $gpiob.pb14.into_pull_up_input(&mut $gpiob.crh),
        button_west: $gpiob.pb15.into_pull_up_input(&mut $gpiob.crh),

        button_l1: $gpioc.pc8.into_pull_up_input(&mut $gpioc.crh),
        button_r1: $gpioc.pc7.into_pull_up_input(&mut $gpioc.crl),

        button_l2: $gpiob.pb11.into_pull_up_input(&mut $gpiob.crh),
        button_r2: $gpiob.pb12.into_pull_up_input(&mut $gpiob.crh),

        button_l3: $gpioc.pc12.into_pull_up_input(&mut $gpioc.crh),
        button_r3: $gpiod.pd2.into_pull_up_input(&mut $gpiod.crl),

        button_home: $gpioc.pc15.into_pull_up_input(&mut $gpioc.crh),
        button_start: $gpioc.pc4.into_pull_up_input(&mut $gpioc.crl),
        button_select: $gpiob.pb10.into_pull_up_input(&mut $gpiob.crh),
        button_trackpad: $gpiob.pb5.into_pull_up_input(&mut $gpiob.crl),

        mode_lock: $gpioc.pc14.into_pull_up_input(&mut $gpioc.crh),
        mode_ls: $gpioc.pc13.into_pull_up_input(&mut $gpioc.crh),
        mode_rs: $gpioc.pc10.into_pull_up_input(&mut $gpioc.crh),
        mode_ps3: $gpioc.pc11.into_pull_up_input(&mut $gpioc.crh),
      }
    }};
  }

  pub struct LedPins {
    pub front: PA6<Output<PushPull>>,
    pub pcb_r: Option<PA7<Output<PushPull>>>,
    pub pcb_g: Option<PB0<Output<PushPull>>>,
    pub pcb_b: Option<PB1<Output<PushPull>>>,
  }

  macro_rules! assign_leds {
    ($gpioa: expr, $gpiob: expr, $gpioc: expr, $gpiod: expr) => {{
      LedPins {
        front: $gpioa.pa6.into_push_pull_output(&mut $gpioa.crl),
        pcb_r: Some($gpioa.pa7.into_push_pull_output(&mut $gpioa.crl)),
        pcb_g: Some($gpiob.pb0.into_push_pull_output(&mut $gpiob.crl)),
        pcb_b: Some($gpiob.pb1.into_push_pull_output(&mut $gpiob.crl)),
      }
    }}
  }
}

// Bluepill
#[cfg(feature = "bluepill")]
#[macro_use]
mod detail {
  use stm32f1xx_hal::gpio::gpioa::*;
  use stm32f1xx_hal::gpio::gpiob::*;
  use stm32f1xx_hal::gpio::gpioc::*;
  use stm32f1xx_hal::gpio::{Input, Output, PullUp, PushPull};

  pub struct InputPins {
    pub stick_down: PB9<Input<PullUp>>,
    pub stick_up: PB8<Input<PullUp>>,
    pub stick_left: PB7<Input<PullUp>>,
    pub stick_right: PB6<Input<PullUp>>,

    pub button_north: PC14<Input<PullUp>>,
    pub button_east: PC15<Input<PullUp>>,
    pub button_south: PA0<Input<PullUp>>,
    pub button_west: PA1<Input<PullUp>>,

    pub button_l1: PA4<Input<PullUp>>,
    pub button_r1: PA5<Input<PullUp>>,

    pub button_l2: PA6<Input<PullUp>>,
    pub button_r2: PA7<Input<PullUp>>,

    pub button_l3: PB0<Input<PullUp>>,
    pub button_r3: PB1<Input<PullUp>>,

    pub button_home: PB15<Input<PullUp>>,
    pub button_start: PB14<Input<PullUp>>,
    pub button_select: PB13<Input<PullUp>>,
    pub button_trackpad: PB12<Input<PullUp>>,

    pub mode_lock: PB5<Input<PullUp>>,
    pub mode_ls: PA10<Input<PullUp>>,
    pub mode_rs: PA9<Input<PullUp>>,
    pub mode_ps3: PA8<Input<PullUp>>,
  }

  macro_rules! assign_inputs {
    ($gpioa: expr, $gpiob: expr, $gpioc: expr, $gpiod: expr) => {{
      let (_a, _b, _c, _d) = (&mut $gpioa, &mut $gpiob, &mut $gpioc, &mut $gpiod);
      InputPins {
        stick_down: $gpiob.pb9.into_pull_up_input(&mut $gpiob.crh),
        stick_up: $gpiob.pb8.into_pull_up_input(&mut $gpiob.crh),
        stick_left: $gpiob.pb7.into_pull_up_input(&mut $gpiob.crl),
        stick_right: $gpiob.pb6.into_pull_up_input(&mut $gpiob.crl),

        button_north: $gpioc.pc14.into_pull_up_input(&mut $gpioc.crh),
        button_east: $gpioc.pc15.into_pull_up_input(&mut $gpioc.crh),
        button_south: $gpioa.pa0.into_pull_up_input(&mut $gpioa.crl),
        button_west: $gpioa.pa1.into_pull_up_input(&mut $gpioa.crl),

        button_l1: $gpioa.pa4.into_pull_up_input(&mut $gpioa.crl),
        button_r1: $gpioa.pa5.into_pull_up_input(&mut $gpioa.crl),

        button_l2: $gpioa.pa6.into_pull_up_input(&mut $gpioa.crl),
        button_r2: $gpioa.pa7.into_pull_up_input(&mut $gpioa.crl),

        button_l3: $gpiob.pb0.into_pull_up_input(&mut $gpiob.crl),
        button_r3: $gpiob.pb1.into_pull_up_input(&mut $gpiob.crl),

        button_home: $gpiob.pb15.into_pull_up_input(&mut $gpiob.crh),
        button_start: $gpiob.pb14.into_pull_up_input(&mut $gpiob.crh),
        button_select: $gpiob.pb13.into_pull_up_input(&mut $gpiob.crh),
        button_trackpad: $gpiob.pb12.into_pull_up_input(&mut $gpiob.crh),

        mode_lock: $gpiob.pb5.into_pull_up_input(&mut $gpiob.crl),
        mode_ls: $gpioa.pa10.into_pull_up_input(&mut $gpioa.crh),
        mode_rs: $gpioa.pa9.into_pull_up_input(&mut $gpioa.crh),
        mode_ps3: $gpioa.pa8.into_pull_up_input(&mut $gpioa.crh),
      }
    }};
  }

  pub struct LedPins {
    pub front: PC13<Output<PushPull>>,
    pub pcb_r: Option<PAx<Output<PushPull>>>,
    pub pcb_g: Option<PAx<Output<PushPull>>>,
    pub pcb_b: Option<PAx<Output<PushPull>>>,
  }

  macro_rules! assign_leds {
    ($gpioa: expr, $gpiob: expr, $gpioc: expr, $gpiod: expr) => {{
      LedPins {
        front: $gpioc.pc13.into_push_pull_output(&mut $gpioc.crh),
        pcb_r: None,
        pcb_g: None,
        pcb_b: None,
      }
    }}
  }
}
pub use detail::*;
