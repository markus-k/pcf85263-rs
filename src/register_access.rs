use crate::{Error, OffsetMode, Pcf85263a};

pub struct Register;
#[allow(dead_code)]
impl Register {
    pub const SECONDS_100TH: u8 = 0x00;
    pub const SECONDS: u8 = 0x01;
    pub const MINUTES: u8 = 0x02;
    pub const HOURS: u8 = 0x03;
    pub const DAYS: u8 = 0x04;
    pub const WEEKDAYS: u8 = 0x05;
    pub const MONTHS: u8 = 0x06;
    pub const YEARS: u8 = 0x07;

    pub const OFFSET: u8 = 0x24;
    pub const OSCILLATOR: u8 = 0x25;
    pub const BATTERY_SWITCH: u8 = 0x26;
    pub const PIN_IO: u8 = 0x27;
    pub const FUNCTION: u8 = 0x28;
    pub const INTA_ENABLE: u8 = 0x29;
    pub const INTB_ENABLE: u8 = 0x2A;
    pub const FLAGS: u8 = 0x2B;

    pub const RAM_BYTE: u8 = 0x2C;

    pub const WATCHDOG: u8 = 0x2D;
    pub const STOP_ENABLE: u8 = 0x2E;
    pub const RESETS: u8 = 0x2F;
}

#[derive(Debug)]
pub enum LoadCapacitance {
    Cl7pF,
    Cl6pF,
    Cl12_5pF,
    Cl12_5pF2,
}

impl From<u8> for LoadCapacitance {
    fn from(val: u8) -> Self {
        match val & 0b11 {
            0b00 => Self::Cl7pF,
            0b01 => Self::Cl6pF,
            0b10 => Self::Cl12_5pF,
            0b11 => Self::Cl12_5pF2,
            _ => unreachable!(),
        }
    }
}

impl LoadCapacitance {
    pub fn as_u8(self) -> u8 {
        match self {
            LoadCapacitance::Cl7pF => 0b00,
            LoadCapacitance::Cl6pF => 0b01,
            LoadCapacitance::Cl12_5pF => 0b10,
            LoadCapacitance::Cl12_5pF2 => 0b11,
        }
    }
}

#[derive(Debug)]
pub struct OscillatorReg(u8);

impl OscillatorReg {
    pub const CLKIV: u8 = 7;
    pub const OFFM: u8 = 6;
    pub const CLK_12_24: u8 = 5;
    pub const LOWJ: u8 = 4;
    pub const OSCD: u8 = 2;
    pub const OSCD_MASK: u8 = 0b11;
    pub const CL: u8 = 0;
    pub const CL_MASK: u8 = 0b11;

    pub fn is_12h_clock(&self) -> bool {
        self.0 & (1 << Self::CLK_12_24) > 0
    }

    pub fn load_capcitance(&self) -> LoadCapacitance {
        LoadCapacitance::from(self.0 & Self::CL_MASK)
    }

    pub fn with_load_capacitance(self, lc: LoadCapacitance) -> Self {
        Self((self.0 & !Self::CL_MASK) | lc.as_u8())
    }

    pub fn with_offset_mode(self, offm: OffsetMode) -> Self {
        match offm {
            OffsetMode::Normal => Self(self.0 & !(1 << Self::OFFM)),
            OffsetMode::Fast => Self(self.0 | (1 << Self::OFFM)),
        }
    }

    pub fn with_low_jitter(self, enabled: bool) -> Self {
        if enabled {
            Self(self.0 | (1 << Self::LOWJ))
        } else {
            Self(self.0 & !(1 << Self::LOWJ))
        }
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl Default for OscillatorReg {
    fn default() -> Self {
        Self(0x00)
    }
}

#[derive(Debug)]
pub enum ClockOutputFrequency {
    F32768,
    F16384,
    F8192,
    F4096,
    F2048,
    F1024,
    F1,
    StaticLow,
}

impl From<u8> for ClockOutputFrequency {
    fn from(val: u8) -> Self {
        match val & 0b11 {
            0b000 => Self::F32768,
            0b001 => Self::F16384,
            0b010 => Self::F8192,
            0b011 => Self::F4096,
            0b100 => Self::F2048,
            0b101 => Self::F1024,
            0b110 => Self::F1,
            0b111 => Self::StaticLow,
            _ => unreachable!(),
        }
    }
}

impl ClockOutputFrequency {
    pub fn as_u8(self) -> u8 {
        match self {
            ClockOutputFrequency::F32768 => 0b000,
            ClockOutputFrequency::F16384 => 0b001,
            ClockOutputFrequency::F8192 => 0b010,
            ClockOutputFrequency::F4096 => 0b011,
            ClockOutputFrequency::F2048 => 0b100,
            ClockOutputFrequency::F1024 => 0b101,
            ClockOutputFrequency::F1 => 0b110,
            ClockOutputFrequency::StaticLow => 0b111,
        }
    }
}

#[derive(Debug)]
pub struct FunctionReg(u8);

impl FunctionReg {
    pub const S_100TH: u8 = 7;
    pub const COF: u8 = 0;
    pub const COF_MASK: u8 = 0b111;

    pub fn s100th_enabled(&self) -> bool {
        self.0 & Self::S_100TH > 0
    }

    pub fn with_100th(self, enable: bool) -> Self {
        if enable {
            Self(self.0 | (1 << Self::S_100TH))
        } else {
            Self(self.0 & !(1 << Self::S_100TH))
        }
    }

    pub fn clock_output_frequency(&self) -> ClockOutputFrequency {
        ClockOutputFrequency::from((self.0 >> Self::COF) & Self::COF_MASK)
    }

    pub fn with_clock_output_frequency(self, cof: ClockOutputFrequency) -> Self {
        Self(self.0 & ((!Self::COF_MASK) << Self::COF) | cof.as_u8() << Self::COF)
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

impl Default for FunctionReg {
    fn default() -> Self {
        Self(0x00)
    }
}

impl<I, E> Pcf85263a<I>
where
    I: RegisterAccess<Error = E>,
{
    pub(crate) fn write_register(&mut self, register: u8, value: u8) -> Result<(), Error<E>> {
        self.interface
            .write_register(register, value)
            .map_err(Error::Interface)
    }

    pub(crate) fn write_register_multiple(
        &mut self,
        start_register: u8,
        values: &[u8],
    ) -> Result<(), Error<E>> {
        self.interface
            .write_registers(start_register, values)
            .map_err(Error::Interface)
    }

    pub(crate) fn read_register(&mut self, register: u8) -> Result<u8, Error<E>> {
        self.interface
            .read_register(register)
            .map_err(Error::Interface)
    }

    pub(crate) fn read_register_multiple<const N: usize>(
        &mut self,
        start_register: u8,
    ) -> Result<[u8; N], Error<E>> {
        let mut values = [0u8; N];

        self.interface
            .read_registers(start_register, &mut values)
            .map_err(Error::Interface)
            .and(Ok(values))
    }

    pub fn read_oscillator_register(&mut self) -> Result<OscillatorReg, Error<E>> {
        Ok(OscillatorReg(self.read_register(Register::OSCILLATOR)?))
    }

    pub fn read_function_register(&mut self) -> Result<FunctionReg, Error<E>> {
        Ok(FunctionReg(self.read_register(Register::FUNCTION)?))
    }

    pub fn write_oscillator_register(&mut self, osc: OscillatorReg) -> Result<(), Error<E>> {
        self.write_register(Register::OSCILLATOR, osc.as_u8())
    }

    pub fn write_stop_register(&mut self, stop: bool) -> Result<(), Error<E>> {
        self.write_register(Register::STOP_ENABLE, if stop { 1 } else { 0 })
    }

    pub fn clear_prescaler(&mut self) -> Result<(), Error<E>> {
        self.write_register(Register::RESETS, 0xA4)
    }

    pub fn write_offset_register(&mut self, offset: i8) -> Result<(), Error<E>> {
        self.write_register(Register::OFFSET, offset.to_be_bytes()[0])
    }

    pub fn write_function_register(&mut self, fr: FunctionReg) -> Result<(), Error<E>> {
        self.write_register(Register::FUNCTION, fr.as_u8())
    }
}

pub trait RegisterAccess {
    type Error;

    fn write_register(&mut self, register: u8, value: u8) -> Result<(), Self::Error>;
    fn write_registers(&mut self, start_register: u8, values: &[u8]) -> Result<(), Self::Error>;

    fn read_register(&mut self, register: u8) -> Result<u8, Self::Error>;
    fn read_registers(&mut self, start_register: u8, values: &mut [u8]) -> Result<(), Self::Error>;
}

pub struct I2cInterface<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C> I2cInterface<I2C> {
    pub fn new(i2c: I2C, address: u8) -> Self {
        Self { i2c, address }
    }

    pub fn release(self) -> I2C {
        self.i2c
    }
}

impl<I2C, E> RegisterAccess for I2cInterface<I2C>
where
    I2C: embedded_hal::i2c::I2c<Error = E>,
{
    type Error = E;

    fn write_register(&mut self, register: u8, value: u8) -> Result<(), Self::Error> {
        let payload = [register, value];

        self.i2c.write(self.address, &payload)
    }

    fn write_registers(&mut self, start_register: u8, values: &[u8]) -> Result<(), Self::Error> {
        // TODO make this more efficient using a single write

        for (register, value) in values
            .into_iter()
            .enumerate()
            .map(|(reg, &value)| (reg as u8 + start_register, value))
        {
            self.write_register(register, value)?;
        }

        Ok(())
    }

    fn read_register(&mut self, register: u8) -> Result<u8, Self::Error> {
        let mut value = [0u8; 1];

        self.read_registers(register, &mut value)?;

        Ok(value[0])
    }

    fn read_registers(&mut self, start_register: u8, values: &mut [u8]) -> Result<(), Self::Error> {
        self.i2c.write_read(self.address, &[start_register], values)
    }
}

#[cfg(test)]
mod tests {
    use crate::DEFAULT_ADDRESS;

    use super::*;
    use embedded_hal_mock::eh1::i2c::{Mock as I2cMock, Transaction as I2cTransaction};

    #[test]
    fn test_write_register() {
        let expectations = [I2cTransaction::write(DEFAULT_ADDRESS, vec![0x12, 0x34])];

        let i2c = I2cMock::new(&expectations);

        let mut rtc = I2cInterface::new(i2c, DEFAULT_ADDRESS);
        rtc.write_register(0x12, 0x34).unwrap();

        let mut i2c = rtc.release();

        i2c.done();
    }

    #[test]
    fn test_read_register() {
        let expectations = [I2cTransaction::write_read(
            DEFAULT_ADDRESS,
            vec![0x12],
            vec![0x34],
        )];

        let i2c = I2cMock::new(&expectations);

        let mut rtc = I2cInterface::new(i2c, DEFAULT_ADDRESS);
        let reg_val = rtc.read_register(0x12).unwrap();
        assert_eq!(reg_val, 0x34);

        let mut i2c = rtc.release();

        i2c.done();
    }

    #[test]
    fn test_read_register_multiple() {
        let expectations = [I2cTransaction::write_read(
            DEFAULT_ADDRESS,
            vec![0x12],
            vec![0x34, 0x56, 0x78],
        )];

        let i2c = I2cMock::new(&expectations);

        let mut rtc = I2cInterface::new(i2c, DEFAULT_ADDRESS);
        let mut reg_val: [u8; 3] = [0; 3];
        rtc.read_registers(0x12, &mut reg_val).unwrap();
        assert_eq!(reg_val[0], 0x34);
        assert_eq!(reg_val[1], 0x56);
        assert_eq!(reg_val[2], 0x78);

        let mut i2c = rtc.release();

        i2c.done();
    }
}
