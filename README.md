## Passing Link

Passing Link is an implementation of a PS4-compatible USB input device on the inexpensive STM32F103
microcontroller, for which entire Blue Pill development boards can be purchased for under $2 shipped.

Currently, support for all inputs except for touchpad gestures and gyro have been implemented. The
PS4 controller authentication scheme has not yet been implemented, so the controller must be reset
every 10 minutes to be used with an actual PS4.

### Goals
Short term:
- [ ] PS4 authentication via extracted private key

Long-term:
- [ ] PS4 audio support
- [ ] USB HID input (to support arcade cabinets with fixed controls overridable by USB input)
- [ ] Custom PCB design

Maybe:
- [ ] PS4 authentication via passthrough to real PS4 controller

Probably not:
- [ ] Flashing firmware via USB (probably won't fit on 64kB flash variants)

### Getting started

#### Acquiring hardware
You'll need two pieces of equipment: the actual development board, and an ST-LINK programmer
with which to flash firmware.

Blue Pills can be purchased from various sources, but be aware that some sources (*especially*
resellers on Amazon) come with an incorrect resistor value that can break USB.

I've had success with [this seller on AliExpress](https://www.aliexpress.com/item/32649400326.html) for a vanilla
Blue Pill, and RobotDyn, which sells a very high quality variant ([$8 with Amazon Prime](https://www.amazon.com/gp/product/B077SRGL47),
[$2.99 shipped from China](https://robotdyn.com/stm32f103-stm32-arm-mini-system-dev-board-stm-firmware.html),
or [$4.59 with 128kB of flash](https://robotdyn.com/stm32f103cbt6-128-kb-flash-stm32-arm-mini-system-dev-board-5d4f1f17-d44f-11e7-b464-10c37b90f38d.html)).

The firmware image doesn't fit in 64kB when compiled in debug mode, and they're dirt cheap, so
ordering a 128kB version from RobotDyn is probably a good idea.

The ST-LINK bundle from the AliExpress seller above works fine, and so does [this one on Amazon](https://www.amazon.com/dp/B01J7N3RE6),
if you'd rather not wait 3 weeks for it to arrive from China.

#### Building and flashing firmware

The following instructions probably work on a recent Ubuntu:

Initial setup:
```
sudo apt-get install openocd gdb-multiarch
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain nightly
source ~/.cargo.env
rustup target add thumbv7m-none-eabi
```

Building an image:
```
cargo -Z config-profile build --release
```

Flashing an image:
```
openocd -f openocd.cfg                   # once, in another terminal
cargo -Z config-profile run --release
```

Debugging:
```
openocd -f openocd.cfg                   # once, in another terminal
gdb-multiarch -q -x openocd.gdb target/thumbv7m-none-eabi/release/passinglink

# Serial output on pin A2, at 921600 baud 8n1
```

#### Pinout

Starting from the top right of the board, going counter-clockwise:

| Pin | Analog | Digital | PWM | Notes
|-----| :----: | :-----: | :-: |------------------------------------------------------------
| A0  |  ADC0  |    ✓    |  ✓  |
| A1  |  ADC1  |    ✓    |  ✓  |
| A2  |  ADC2  |    ✓    |  ✓  | Serial console output, avoid
| A3  |  ADC3  |    ✓    |  ✓  | Serial console input, currently unused, avoid unless needed
| A4  |  ADC4  |    ✓    |  ✗  |
| A5  |  ADC5  |    ✓    |  ✗  |
| A6  |  ADC6  |    ✓    |  ✓  |
| A7  |  ADC7  |    ✓    |  ✓  |
| B0  |  ADC8  |    ✓    |  ✓  |
| B1  |  ADC9  |    ✓    |  ✓  |
| B10 |   ✗    |    ✓    |  ✗  |
| B11 |   ✗    |    ✓    |  ✗  |
|     |        |         |     |
| B12 |   ✗    |    ✓    |  ✗  |
| B13 |   ✗    |    ✓    |  ✗  |
| B14 |   ✗    |    ✓    |  ✗  |
| B15 |   ✗    |    ✓    |  ✗  |
| A8  |   ✗    |    ✓    |  ✓  |
| A9  |   ✗    |    ✓    |  ✓  |
| A10 |   ✗    |    ✓    |  ✓  |
| A11 |   ☠    |    ☠    |  ☠  | USB pin: do not use this
| A12 |   ☠    |    ☠    |  ☠  | USB pin: do not use this
| A15 |   ✗    |    ✓    |  ✗  |
| B3  |   ✗    |    ✓    |  ✗  |
| B4  |   ✗    |    ✓    |  ✗  |
| B5  |   ✗    |    ✓    |  ✗  |
| B6  |   ✗    |    ✓    |  ✓  |
| B7  |   ✗    |    ✓    |  ✓  |
| B8  |   ✗    |    ✓    |  ✓  |
| B9  |   ✗    |    ✓    |  ✓  |
