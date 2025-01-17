use crate::{
    Result,
    error
};

pub fn parse_vec<const N: usize>(s: &str) -> Result<[u8; N]> {
    if s.len() == 0 || !s.starts_with('[') || !s.ends_with(']') {
        return error("could not parse vec");
    }
    let parts = s[1..s.len()-1].split(',');

    let v: Vec<_> = parts.map(|str| str.parse::<u8>()).collect();
    if v.len() < N || v.iter().any(|val| val.is_err()) {
        return error("could not parse vec");
    }
    let v: Vec<u8> = v.iter().map(|res| *res.as_ref().unwrap()).collect();
    
    let (left, _) = v.split_at(N);
    let left: std::result::Result<[u8; N], _> = left.try_into();
    if left.is_err() {
        return error("could not parse vec");
    }
    Ok(left.unwrap())
}

pub fn parse_u64(mut s: &str) -> Result<u64> {
    let mut base = 10;
    if s.starts_with("0x") {
        s = &s[2..];
        base = 16;
    }
    let mut val = 0;
    for ch in s.chars() {
        val *= base;
        let Ok(res) = parse_digit(ch, base) else {
            return error("could not parse integer");
        };
        val += res;
    }
    Ok(val)
}

pub fn parse_float(s: &str) -> Result<f64> {
    let res = s.parse::<f64>();
    if res.is_err() {
        return error("could not parse float");
    }
    Ok(res.unwrap())
}

fn parse_digit_b10(ch: char) -> Result<u64> {
    if ch < '0' || ch > '9' {
        return error("error parsing integer");
    }
    return Ok((ch as u8 - '0' as u8) as u64);
}

fn parse_digit(ch: char, base: u64) -> Result<u64> {
    let res = parse_digit_b10(ch);
    if res.is_ok() {
        return res;
    }
    if base == 16 {
        if ch >= 'a' && ch <= 'f' {
            return Ok((ch as u8 - 'a' as u8) as u64 + 10);
        }
        if ch >= 'A' && ch <= 'F' {
            return Ok((ch as u8 - 'A' as u8) as u64 + 10);
        }
    }
    return error("could not parse digit");
}