#[derive(PartialEq)]
enum Coord {
    Lat,
    Lng,
}

/// Decodes a H02 lat/lng string to its decimal representation.
///
/// A H02 lat/lng string has the first 2 or 3 digits represent the degrees
/// and the following digits be the minutes, both in decimal format, this can
/// fail if the string is invalid, incomplete or represents a invalid lat/lng.
///  
/// # Arguments
///
/// * `s` - The string to convert
/// * `degree_digits` - Number of first digits to consider as degrees
///
/// # Examples
///
/// ```
/// // "20" is considered the degrees
/// // "27.93290" is considered the minutes (0.465548 when converted to decimal)
/// str_to_coord("2027.93290", Coord::Lat).unwrap() // 20.465548
/// ```
fn str_to_coord(s: &str, to: Coord) -> Result<f64, String> {
    let degree_digits = match to {
        Coord::Lat => 2,
        Coord::Lng => 3,
    };

    if s.len() < degree_digits {
        return Err("cannot decode point string, not enough degree digits".to_string());
    }

    let degrees = &s[..degree_digits]
        .parse::<f64>()
        .or(Err("failed to parse point degrees"))?;

    if to == Coord::Lat && (degrees < &-90.0 || degrees > &90.0) {
        return Err("latitude value out of bounds [-90..90]".to_string());
    }

    if to == Coord::Lng && (degrees < &-180.0 || degrees > &180.0) {
        return Err("longitude value out of bounds [-180..180]".to_string());
    }

    let minute_digits = &s[degree_digits..];

    let minutes = if minute_digits.is_empty() {
        0.0
    } else {
        minute_digits
            .parse::<f64>()
            .or(Err("failed to parse point minutes"))?
    };

    if minutes < 0.0 || minutes > 60.0 {
        return Err("point minutes not between bounds [0..60]".to_string());
    }

    Ok(degrees + (minutes / 60.0))
}

pub fn str_to_lat(s: &str) -> Result<f64, String> {
    return str_to_coord(s, Coord::Lat);
}

pub fn str_to_lng(s: &str) -> Result<f64, String> {
    return str_to_coord(s, Coord::Lng);
}
