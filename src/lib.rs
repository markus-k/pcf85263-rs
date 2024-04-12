#![no_std]

mod datetime;
mod register_access;

pub use register_access::RegisterAccess;
pub use rtcc::{DateTimeAccess, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

pub use crate::register_access::{
    ClockOutputFrequency, FunctionReg, I2cInterface, LoadCapacitance, OscillatorReg,
};

pub const DEFAULT_ADDRESS: u8 = 0x51; // 0xA2 (W) + 0xA3 (R)

#[derive(Debug)]
pub enum Error<E> {
    Interface(E),
    InvalidDate,
}

pub enum OffsetMode {
    /// Correction made every 4 hours, 2.170ppm/step
    Normal,
    /// Correction made every 8 minutes, 2.0345ppm/step
    Fast,
}

impl OffsetMode {
    /// Offset per step, measured in 0.1ppm/step
    pub const fn offset_per_step(&self) -> u32 {
        match self {
            OffsetMode::Normal => 21700,
            OffsetMode::Fast => 20345,
        }
    }
}

pub struct Pcf85263a<I> {
    interface: I,
}

impl<I, E> Pcf85263a<I>
where
    I: RegisterAccess<Error = E>,
{
    pub fn new(interface: I) -> Self {
        Pcf85263a { interface }
    }

    pub fn release(self) -> I {
        self.interface
    }
}

impl<I2C, E> Pcf85263a<I2cInterface<I2C>>
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    pub fn new_with_i2c(i2c: I2C) -> Self {
        Self::new(I2cInterface::new(i2c, DEFAULT_ADDRESS))
    }
}
