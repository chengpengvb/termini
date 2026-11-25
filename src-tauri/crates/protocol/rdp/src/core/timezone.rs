use crate::core::stream::WriteCursor;
use crate::core::error::{RdpProtocolError, RdpResult};

const TIME_ZONE_NAME_UNITS: usize = 32;
#[cfg(windows)]
const TIME_ZONE_KEY_NAME_UNITS: usize = 128;
#[cfg(windows)]
const TIME_ZONE_ID_INVALID: u32 = 0xFFFF_FFFF;
#[cfg(not(windows))]
const SECONDS_PER_MINUTE: i32 = 60;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SystemTime {
    pub year: u16,
    pub month: u16,
    pub day_of_week: u16,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
    pub milliseconds: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TimeZoneInformation {
    pub bias: i32,
    pub standard_name: [u16; TIME_ZONE_NAME_UNITS],
    pub standard_date: SystemTime,
    pub standard_bias: i32,
    pub daylight_name: [u16; TIME_ZONE_NAME_UNITS],
    pub daylight_date: SystemTime,
    pub daylight_bias: i32,
}

impl Default for TimeZoneInformation {
    fn default() -> Self {
        Self {
            bias: 0,
            standard_name: [0; TIME_ZONE_NAME_UNITS],
            standard_date: SystemTime::default(),
            standard_bias: 0,
            daylight_name: [0; TIME_ZONE_NAME_UNITS],
            daylight_date: SystemTime::default(),
            daylight_bias: 0,
        }
    }
}

pub fn write_time_zone_information(cursor: &mut WriteCursor) -> RdpResult<()> {
    write_time_zone(cursor, &local_time_zone()?);
    Ok(())
}

pub fn write_time_zone(cursor: &mut WriteCursor, tz: &TimeZoneInformation) {
    cursor.write_u32_le(tz.bias as u32);
    write_utf16_units(cursor, &tz.standard_name);
    write_system_time(cursor, &tz.standard_date);
    cursor.write_u32_le(tz.standard_bias as u32);
    write_utf16_units(cursor, &tz.daylight_name);
    write_system_time(cursor, &tz.daylight_date);
    cursor.write_u32_le(tz.daylight_bias as u32);
}

pub fn write_dynamic_time_zone(cursor: &mut WriteCursor) -> RdpResult<()> {
    let (key_name, daylight_disabled) = local_dynamic_time_zone()?;
    cursor.write_u16_le(0);
    cursor.write_u16_le(0);
    let length = key_name.encode_utf16().count() * 2;
    cursor.write_u16_le(u16::try_from(length).map_err(|_| RdpProtocolError::encoding(format!("dynamic time zone name too long: {}", length)))?);
    cursor.write_utf16_le(&key_name);
    cursor.write_u16_le(u16::from(daylight_disabled));
    Ok(())
}

fn write_utf16_units(cursor: &mut WriteCursor, units: &[u16; TIME_ZONE_NAME_UNITS]) {
    for unit in units {
        cursor.write_u16_le(*unit);
    }
}

fn write_system_time(cursor: &mut WriteCursor, time: &SystemTime) {
    cursor.write_u16_le(time.year);
    cursor.write_u16_le(time.month);
    cursor.write_u16_le(time.day_of_week);
    cursor.write_u16_le(time.day);
    cursor.write_u16_le(time.hour);
    cursor.write_u16_le(time.minute);
    cursor.write_u16_le(time.second);
    cursor.write_u16_le(time.milliseconds);
}

#[cfg(windows)]
fn utf16_zstring(units: &[u16]) -> RdpResult<String> {
    let end = units.iter().position(|unit| *unit == 0).unwrap_or(units.len());
    String::from_utf16(&units[..end]).map_err(|error| RdpProtocolError::encoding(format!("invalid time zone name: {}", error)))
}

#[cfg(windows)]
fn local_time_zone() -> RdpResult<TimeZoneInformation> {
    let mut info = TimeZoneInformation::default();
    unsafe {
        if GetTimeZoneInformation(&mut info) == TIME_ZONE_ID_INVALID {
            return Err(RdpProtocolError::protocol(format!("GetTimeZoneInformation failed: {}", GetLastError())));
        }
    }
    Ok(info)
}

#[cfg(windows)]
fn local_dynamic_time_zone() -> RdpResult<(String, bool)> {
    let mut info = DynamicTimeZoneInformation::default();
    unsafe {
        if GetDynamicTimeZoneInformation(&mut info) == TIME_ZONE_ID_INVALID {
            return Err(RdpProtocolError::protocol(format!("GetDynamicTimeZoneInformation failed: {}", GetLastError())));
        }
    }
    Ok((utf16_zstring(&info.time_zone_key_name)?, info.dynamic_daylight_time_disabled != 0))
}

#[cfg(not(windows))]
fn local_time_zone() -> RdpResult<TimeZoneInformation> {
    use chrono::Local;
    use chrono::Offset;

    let name = iana_time_zone::get_timezone().map_err(|error| RdpProtocolError::protocol(format!("failed to read local time zone: {}", error)))?;
    let mut units = [0u16; TIME_ZONE_NAME_UNITS];
    for (index, unit) in name.encode_utf16().take(TIME_ZONE_NAME_UNITS - 1).enumerate() {
        units[index] = unit;
    }
    Ok(TimeZoneInformation {
        bias: -Local::now().offset().fix().local_minus_utc() / SECONDS_PER_MINUTE,
        standard_name: units,
        standard_date: SystemTime::default(),
        standard_bias: 0,
        daylight_name: units,
        daylight_date: SystemTime::default(),
        daylight_bias: 0,
    })
}

#[cfg(not(windows))]
fn local_dynamic_time_zone() -> RdpResult<(String, bool)> {
    Ok((String::new(), false))
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct DynamicTimeZoneInformation {
    _tz: TimeZoneInformation,
    time_zone_key_name: [u16; TIME_ZONE_KEY_NAME_UNITS],
    dynamic_daylight_time_disabled: u8,
}

#[cfg(windows)]
impl Default for DynamicTimeZoneInformation {
    fn default() -> Self {
        Self { _tz: TimeZoneInformation::default(), time_zone_key_name: [0; TIME_ZONE_KEY_NAME_UNITS], dynamic_daylight_time_disabled: 0 }
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetTimeZoneInformation(info: *mut TimeZoneInformation) -> u32;
    fn GetDynamicTimeZoneInformation(info: *mut DynamicTimeZoneInformation) -> u32;
    fn GetLastError() -> u32;
}
