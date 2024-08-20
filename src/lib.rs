#![cfg_attr(not(test), no_std)]

mod datetime;
mod register_access;

pub use register_access::RegisterAccess;
pub use rtcc::{DateTimeAccess, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

pub use crate::register_access::{
    ClockOutputFrequency, CrystalDrive, FunctionReg, I2cInterface, IntAPinMode, InterruptReg,
    LoadCapacitance, OscillatorReg, PeriodicInterrupt, PinIoReg,
};

pub const DEFAULT_ADDRESS: u8 = 0x51; // 0xA2 (W) + 0xA3 (R)

#[derive(Debug)]
pub enum Error<E> {
    Interface(E),
    InvalidDate,
}

#[derive(Debug, Clone, Copy)]
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

/// Helper function to calculate the offset value for a given offset in ppb.
///
/// This essentially maps Table 24 from the datasheet.
pub fn offset_value_for_ppb_offset(offset_ppb: i32, offset_mode: OffsetMode) -> i8 {
    let tenthppb_per_pulse: i64 = match offset_mode {
        OffsetMode::Normal => 21700,
        OffsetMode::Fast => 20345,
    };

    (((offset_ppb as i64) * 10 + (offset_ppb.signum() as i64 * tenthppb_per_pulse / 2))
        / tenthppb_per_pulse)
        .clamp(i8::MIN as i64, i8::MAX as i64) as i8
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offsets() {
        let table = [
            // a selection of test cases to make sure rounding is
            // working correctly
            (0, 0, OffsetMode::Normal),
            (0, 0, OffsetMode::Fast),
            (2, 5_000, OffsetMode::Normal),
            (0, 600, OffsetMode::Fast),
            (1, 1_100, OffsetMode::Fast),
            (0, -500, OffsetMode::Normal),
            (-1, -2_000, OffsetMode::Normal),
            (126, 256_000, OffsetMode::Fast),
            (127, 300_000, OffsetMode::Normal),
            (-127, -275_600, OffsetMode::Normal),
            (-128, -300_000, OffsetMode::Normal),
        ];

        for test in table {
            let offset = offset_value_for_ppb_offset(test.1, test.2);
            assert_eq!(
                offset, test.0,
                "Offset in ppb: {}, offset: {}, expected: {}, mode: {:?}",
                test.1, offset, test.0, test.2
            );
        }
    }
}
