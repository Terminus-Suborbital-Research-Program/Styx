#![no_std]

use defmt::info;
use embedded_hal::digital::{OutputPin, StatefulOutputPin};
use fugit::ExtU64;
use rp235x_hal::adc::{AdcFifo, AdcPin};
use rp235x_hal::{gpio, pac, Adc, Clock, Sio, Watchdog};
use rp235x_hal::pac::Peripherals;
use rp235x_hal::gpio::{PinState, PullNone};
use rtic_monotonics::Monotonic;

pub enum MpChannel
{
    PD1_4 = 0b00000, // LOW: GPIO 8 9 10
    PD5_8 = 0b00001, // GPIO 8
    PD9_12 = 0b00010, // GPIO 9
    PD13_16 = 0b00011, // GPIO 8 9
    PD17_20 = 0b00100, // GPIO 10
    PD21_24 = 0b00101, // GPIO 8 10
}

pub struct PDMuxController
{
    pub pin19: gpio::Pin<gpio::bank0::Gpio19, gpio::FunctionSio<gpio::SioOutput>, gpio::PullNone>,
    pub pin20: gpio::Pin<gpio::bank0::Gpio20, gpio::FunctionSio<gpio::SioOutput>, gpio::PullNone>,
    pub pin21: gpio::Pin<gpio::bank0::Gpio21, gpio::FunctionSio<gpio::SioOutput>, gpio::PullNone>,

    pub disable_pin: gpio::Pin<gpio::bank0::Gpio12, gpio::FunctionSio<gpio::SioOutput>, gpio::PullNone>,

    pub adc_pin_0: AdcPin<gpio::Pin<gpio::bank0::Gpio14, gpio::FunctionSio<gpio::SioInput>, gpio::PullNone>>,
    pub adc_pin_1: AdcPin<gpio::Pin<gpio::bank0::Gpio13, gpio::FunctionSio<gpio::SioInput>, gpio::PullNone>>,
    pub adc_pin_2: AdcPin<gpio::Pin<gpio::bank0::Gpio11, gpio::FunctionSio<gpio::SioInput>, gpio::PullNone>>,
    pub adc_pin_3: AdcPin<gpio::Pin<gpio::bank0::Gpio10, gpio::FunctionSio<gpio::SioInput>, gpio::PullNone>>,

    pub adc: &'static mut Adc,
    pub adc_outputs: [u16; 24],

    pub channel: MpChannel,
    i: usize,
}

impl PDMuxController
{
    pub fn new(
        pin19: gpio::Pin<gpio::bank0::Gpio19, gpio::FunctionSio<gpio::SioOutput>, gpio::PullNone>,
        pin20: gpio::Pin<gpio::bank0::Gpio20, gpio::FunctionSio<gpio::SioOutput>, gpio::PullNone>,
        pin21: gpio::Pin<gpio::bank0::Gpio21, gpio::FunctionSio<gpio::SioOutput>, gpio::PullNone>,
        disable_pin: gpio::Pin<gpio::bank0::Gpio12, gpio::FunctionSio<gpio::SioOutput>, gpio::PullNone>,
        adc_0: AdcPin<gpio::Pin<gpio::bank0::Gpio14, gpio::FunctionSio<gpio::SioInput>, gpio::PullNone>>,
        adc_1: AdcPin<gpio::Pin<gpio::bank0::Gpio13, gpio::FunctionSio<gpio::SioInput>, gpio::PullNone>>,
        adc_2: AdcPin<gpio::Pin<gpio::bank0::Gpio11, gpio::FunctionSio<gpio::SioInput>, gpio::PullNone>>,
        adc_3: AdcPin<gpio::Pin<gpio::bank0::Gpio10, gpio::FunctionSio<gpio::SioInput>, gpio::PullNone>>,
        adc: &'static mut Adc,
    ) -> PDMuxController
    {

        PDMuxController {
            pin19,
            pin20,
            pin21,

            disable_pin,

            adc_pin_0: adc_0,
            adc_pin_1: adc_1,
            adc_pin_2: adc_2,
            adc_pin_3: adc_3,
            adc,

            adc_outputs: [0; 24],
            channel: MpChannel::PD1_4,
            i: 0,
        }
    }

    pub fn read_photodiodes(&mut self)
    {
        if self.disable_pin.is_set_low().unwrap()
        {
            if (self.i % 4) == 0
            {
                match self.channel
                {
                    MpChannel::PD1_4 => {
                        self.pin19.set_low().unwrap();
                        self.pin20.set_low().unwrap();
                        self.pin21.set_low().unwrap();

                        self.channel = MpChannel::PD5_8;
                    }

                    MpChannel::PD5_8 => {
                        self.pin19.set_high().unwrap();
                        self.pin20.set_low().unwrap();
                        self.pin21.set_low().unwrap();

                        self.channel = MpChannel::PD9_12;
                    }

                    MpChannel::PD9_12 => {
                        self.pin19.set_low().unwrap();
                        self.pin20.set_high().unwrap();
                        self.pin21.set_low().unwrap();

                        self.channel = MpChannel::PD13_16;
                    }

                    MpChannel::PD13_16 => {
                        self.pin19.set_high().unwrap();
                        self.pin20.set_high().unwrap();
                        self.pin21.set_low().unwrap();

                        self.channel = MpChannel::PD17_20;
                    }

                    MpChannel::PD17_20 => {
                        self.pin19.set_low().unwrap();
                        self.pin20.set_low().unwrap();
                        self.pin21.set_high().unwrap();

                        self.channel = MpChannel::PD21_24;
                    }

                    MpChannel::PD21_24 => {
                        self.pin19.set_high().unwrap();
                        self.pin20.set_low().unwrap();
                        self.pin21.set_high().unwrap();

                        self.channel = MpChannel::PD1_4;
                    }
                }
            }

            if (self.i % 4) == 0
            {
                self.adc_outputs[self.i] = self.adc.read(&mut self.adc_pin_0).unwrap();
            } else if (self.i % 4) == 1
            {
                self.adc_outputs[self.i] = self.adc.read(&mut self.adc_pin_1).unwrap();
            } else if (self.i % 4) == 2
            {
                self.adc_outputs[self.i] = self.adc.read(&mut self.adc_pin_2).unwrap();
            } else if (self.i % 4) == 3
            {
                self.adc_outputs[self.i] = self.adc.read(&mut self.adc_pin_3).unwrap();
            }

            info!("Added {} to adc_outputs.", self.adc_outputs[self.i]);

            self.i = self.i + 1;
            self.i = self.i % 24;
        }
    }
}
