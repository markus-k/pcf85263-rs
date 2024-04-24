use crate::register_access::{OscillatorReg, Register, RegisterAccess};
use crate::{Error, Pcf85263a};

use rtcc::{DateTimeAccess, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

impl<I, E> DateTimeAccess for Pcf85263a<I>
where
    I: RegisterAccess<Error = E>,
{
    type Error = Error<E>;

    fn datetime(&mut self) -> Result<NaiveDateTime, Self::Error> {
        self.datetime()
    }

    fn set_datetime(&mut self, datetime: &NaiveDateTime) -> Result<(), Self::Error> {
        self.set_datetime(datetime)
    }
}

impl<I, E> Pcf85263a<I>
where
    I: RegisterAccess<Error = E>,
{
    pub fn time(&mut self) -> Result<NaiveTime, Error<E>> {
        let [seconds_100th, seconds, minutes, hours] =
            self.read_register_multiple(Register::SECONDS_100TH)?;
        let osc_reg = self.read_oscillator_register()?; // TODO should probably get rid of this..

        let hour = decode_hours(hours, osc_reg).as_24h().into();
        let minute = decode_minutes(minutes).into();
        let second = decode_seconds(seconds).into();
        let millisecond = (decode_seconds_100th(seconds_100th) as u32 * 10).min(999);

        Ok(NaiveTime::from_hms_milli_opt(hour, minute, second, millisecond).unwrap())
    }

    pub fn set_time(&mut self, time: NaiveTime) -> Result<(), Error<E>> {
        let osc_reg = self.read_oscillator_register()?;
        // see datasheet page 14
        self.write_stop_register(true)?;
        self.clear_prescaler()?;
        self.write_register_multiple(
            Register::SECONDS_100TH,
            &[
                0,
                encode_bcd(time.second() as u8),
                encode_bcd(time.minute() as u8),
                encode_hours(time.hour() as u8, osc_reg),
            ],
        )?;
        self.write_stop_register(false)?;

        Ok(())
    }

    pub fn date(&mut self) -> Result<NaiveDate, Error<E>> {
        let [days, _weekdays, months, years] = self.read_register_multiple(Register::DAYS)?;

        Ok(NaiveDate::from_ymd_opt(
            decode_years(years).into(),
            decode_months(months).into(),
            decode_days(days).into(),
        )
        .unwrap())
    }

    pub fn set_date(&mut self, date: NaiveDate) -> Result<(), Error<E>> {
        self.write_stop_register(true)?;

        self.write_register(Register::DAYS, encode_bcd(date.day() as u8))?;
        self.write_register(Register::MONTHS, encode_bcd(date.month() as u8))?;
        self.write_register(Register::YEARS, encode_years(date.year())?)?;

        self.write_stop_register(false)?;

        Ok(())
    }

    pub fn datetime(&mut self) -> Result<NaiveDateTime, Error<E>> {
        Ok(self.date()?.and_time(self.time()?))
    }

    pub fn set_datetime(&mut self, datetime: &NaiveDateTime) -> Result<(), Error<E>> {
        self.set_date(datetime.date())?;
        self.set_time(datetime.time())?;

        Ok(())
    }
}

fn decode_seconds(val: u8) -> u8 {
    decode_bcd(val & 0b01111111)
}

fn decode_minutes(val: u8) -> u8 {
    decode_bcd(val & 0b01111111)
}

fn decode_seconds_100th(val: u8) -> u8 {
    decode_bcd(val)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Hours {
    AM(u8),
    PM(u8),
    H24(u8),
}

impl Hours {
    pub fn as_24h(self) -> u8 {
        // wtf is wrong with you americans
        match self {
            Hours::H24(hour) => hour,
            Hours::AM(am) => {
                if am == 12 {
                    0
                } else {
                    am
                }
            }
            Hours::PM(pm) => {
                if pm == 12 {
                    12
                } else {
                    pm + 12
                }
            }
        }
    }

    pub fn from_24h_as_ampm(hour: u8) -> Self {
        if hour <= 11 {
            if hour == 0 {
                Self::AM(12)
            } else {
                Self::AM(hour)
            }
        } else {
            if hour == 12 {
                Self::PM(12)
            } else {
                Self::PM(hour - 12)
            }
        }
    }
}

fn decode_hours(hours: u8, osc_reg: OscillatorReg) -> Hours {
    if osc_reg.is_12h_clock() {
        let h12_hour = decode_bcd(hours & 0b00011111);
        if hours & (1 << 5) > 0 {
            Hours::AM(h12_hour)
        } else {
            Hours::PM(h12_hour)
        }
    } else {
        Hours::H24(decode_bcd(hours & 0b00111111))
    }
}

fn encode_hours(hours: u8, osc_reg: OscillatorReg) -> u8 {
    if osc_reg.is_12h_clock() {
        let hours = Hours::from_24h_as_ampm(hours);

        match hours {
            Hours::AM(am) => encode_bcd(am) | (1 << 5),
            Hours::PM(pm) => encode_bcd(pm),
            _ => unreachable!(),
        }
    } else {
        encode_bcd(hours)
    }
}

fn encode_years<E>(year: i32) -> Result<u8, Error<E>> {
    if year < 2000 || year >= 3000 {
        Err(Error::InvalidDate)
    } else {
        let year = (2000 - year) as u8;
        Ok(encode_bcd(year))
    }
}

fn decode_days(days: u8) -> u8 {
    decode_bcd(days & 0b00111111)
}

fn decode_months(months: u8) -> u8 {
    decode_bcd(months & 0b00000111)
}

fn decode_years(years: u8) -> u16 {
    decode_bcd(years) as u16 + 2000
}

fn decode_bcd(bcd: u8) -> u8 {
    let unit = bcd & 0xF;
    let tens = (bcd >> 4) & 0xF;

    unit + tens * 10
}

fn encode_bcd(val: u8) -> u8 {
    let unit = val % 10;
    let tens = val / 10;

    unit | (tens << 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_bcd() {
        assert_eq!(decode_bcd(0b00000010), 2);
        assert_eq!(decode_bcd(0b00110000), 30);
        assert_eq!(decode_bcd(0b10011000), 98);
    }

    #[test]
    fn test_encode_bcd() {
        assert_eq!(encode_bcd(2), 0b00000010);
        assert_eq!(encode_bcd(30), 0b00110000);
        assert_eq!(encode_bcd(98), 0b10011000);
    }

    #[test]
    fn test_hours_to_24h() {
        for h in 0..=23 {
            assert_eq!(Hours::H24(h).as_24h(), h);
        }

        assert_eq!(Hours::AM(12).as_24h(), 0);
        for h in 1..=11 {
            assert_eq!(Hours::AM(h).as_24h(), h);
        }

        assert_eq!(Hours::PM(12).as_24h(), 12);
        for h in 1..=11 {
            assert_eq!(Hours::PM(h).as_24h(), h + 12);
        }
    }
}
