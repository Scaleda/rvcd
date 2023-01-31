use crate::wave::WireValue;
use num_bigint::BigUint;
use std::cmp::min;
use tracing::debug;

#[derive(serde::Deserialize, serde::Serialize, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Radix {
    Bin,
    Oct,
    Dec,
    Hex,
}

pub fn radix_vector_to_string(radix: Radix, vec: &Vec<WireValue>) -> String {
    if radix == Radix::Dec {
        radix_vector_dec(vec)
    } else {
        let n: usize = match radix {
            Radix::Bin => 1,
            Radix::Oct => 3,
            Radix::Hex => 4,
            _ => panic!("internal err"),
        };
        radix_vector_to_string_n(vec, n)
    }
}

pub fn radix_vector_bin(vec: &[WireValue]) -> String {
    vec.iter()
        .rev()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join("")
}

fn value_map_val(v: &WireValue) -> u8 {
    match v {
        WireValue::V0 => 0,
        _ => 1,
    }
}

pub fn radix_value_big_uint(vec: &Vec<WireValue>) -> BigUint {
    let bits = vec.iter().map(value_map_val);
    let mut bytes: Vec<u8> = vec![];
    let mut byte = 0_u8;
    bits.enumerate().for_each(|i| {
        let offset = i.0 & 0x7;
        if offset == 0 {
            byte = i.1;
        } else {
            byte |= i.1 << offset;
            if offset == 7 {
                bytes.push(byte);
            }
        }
    });
    if vec.len() & 0x7 != 0 {
        bytes.push(byte);
    }
    // assert!(bytes.len() % 8 < 2);
    BigUint::from_bytes_le(&bytes)
}

pub fn radix_vector_to_string_n(vec: &Vec<WireValue>, n: usize) -> String {
    debug!("n = {}", n);
    assert!(n > 0);
    let val = radix_value_big_uint(vec);
    let mut str = val.to_str_radix(1 << n);
    let bits_should_len = ((vec.len() / n) + usize::from(vec.len() % n != 0)) * n;
    let vec_extended = vec
        .iter()
        .chain((0..(bits_should_len - vec.len())).map(|_| &WireValue::V0))
        .copied()
        .collect::<Vec<_>>();
    debug!(
        "str len={}, vec len={}, str_len<<(n-1)={}, bits_should_len={}",
        str.len(),
        vec.len(),
        str.len() << (n - 1),
        bits_should_len
    );
    let prefix_len = (bits_should_len / n) - str.len();
    let prefix = (0..prefix_len).map(|_| "0").collect::<Vec<_>>().join("");
    debug!("prefix = {}", prefix);
    str = prefix + &str;
    // for every 'z' or 'x' bit,
    // 1. in this 2^n bit have only one 'x' or 'z', then change char as 'x' or 'z'
    // 2. in this 2^n bit have 'x' and 'z', use 'x'
    debug!("str={}", str);
    if !str.is_empty() {
        debug!(
            "vec_extended = {:?}\nrev: {:?}",
            vec_extended,
            vec_extended.iter().rev().collect::<Vec<_>>()
        );
        let indexes_target = |target: WireValue| {
            vec_extended
                .iter()
                .rev()
                .enumerate()
                .filter(|(_, v)| **v == target)
                .map(|i| i.0)
                .collect::<Vec<_>>()
        };
        let indexes_z = indexes_target(WireValue::Z);
        let indexes_x = indexes_target(WireValue::X);
        let mut do_replace = |indexes: Vec<usize>, with: &str| {
            debug!("indexes for {}: {:?}", with, indexes);
            indexes.into_iter().map(|i| i / n).for_each(|i| {
                str.replace_range(min(i, str.len() - 1)..min(i + 1, str.len()), with)
            });
        };
        do_replace(indexes_z, "z");
        do_replace(indexes_x, "x");
    }
    str
}

pub fn radix_vector_dec(vec: &Vec<WireValue>) -> String {
    let val = radix_value_big_uint(vec);
    let str = val.to_str_radix(10);
    let exists_x = vec.contains(&WireValue::X);
    let exists_z = vec.contains(&WireValue::Z);
    if exists_x || exists_z {
        // directly change all chars to x or z
        (0..str.len())
            .map(|_| if exists_x { "x" } else { "z" })
            .collect::<Vec<_>>()
            .join("")
    } else {
        str
    }
}

#[cfg(test)]
mod test {
    use crate::radix::{radix_vector_to_string, Radix};
    use crate::wave::WireValue;
    use crate::wave::WireValue::*;
    use anyhow::Result;

    #[test]
    fn test_vector_string() -> Result<()> {
        let vec: Vec<WireValue> = vec![V1, V1, V1, V0, V0, V1, V1, V0];
        let bin = radix_vector_to_string(Radix::Bin, &vec);
        let oct = radix_vector_to_string(Radix::Oct, &vec);
        let dec = radix_vector_to_string(Radix::Dec, &vec);
        let hex = radix_vector_to_string(Radix::Hex, &vec);
        debug!(
            "vec rev: {:?}, bin={}, oct={}, dec={}, hex={}",
            vec.iter().rev().collect::<Vec<_>>(),
            bin,
            oct,
            dec,
            hex
        );

        let vec: Vec<WireValue> = vec![V1, V1, V1, X, V0, V1, V1, Z];
        let bin = radix_vector_to_string(Radix::Bin, &vec);
        let oct = radix_vector_to_string(Radix::Oct, &vec);
        let dec = radix_vector_to_string(Radix::Dec, &vec);
        let hex = radix_vector_to_string(Radix::Hex, &vec);
        debug!(
            "vec rev: {:?}, bin={}, oct={}, dec={}, hex={}",
            vec.iter().rev().collect::<Vec<_>>(),
            bin,
            oct,
            dec,
            hex
        );

        let vec: Vec<WireValue> = vec![V1, V1, V1, X, V0, V1, X, Z, V0, V0, V0];
        let bin = radix_vector_to_string(Radix::Bin, &vec);
        let oct = radix_vector_to_string(Radix::Oct, &vec);
        let dec = radix_vector_to_string(Radix::Dec, &vec);
        let hex = radix_vector_to_string(Radix::Hex, &vec);
        debug!(
            "vec rev: {:?}, bin={}, oct={}, dec={}, hex={}",
            vec.iter().rev().collect::<Vec<_>>(),
            bin,
            oct,
            dec,
            hex
        );
        Ok(())
    }
}
