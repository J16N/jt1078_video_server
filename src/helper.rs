#![allow(dead_code)]

pub(crate) fn get_bit_at(n: u8, i: usize) -> Result<u8, String> {
    if i > 7 {
        return Err("Index out of bounds".to_string());
    }
    Ok((n >> i) & 1)
}

pub(crate) fn get_num_at(n: u8, index: usize, length: usize) -> Result<u8, String> {
    if index > 7 {
        return Err("Offset out of bounds".to_string());
    }
    let stub = match length {
        1 => 0b0000_0001,
        2 => 0b0000_0011,
        3 => 0b0000_0111,
        4 => 0b0000_1111,
        5 => 0b0001_1111,
        6 => 0b0011_1111,
        7 => 0b0111_1111,
        8 => 0b1111_1111,
        _ => return Err("Invalid length".to_string()),
    };
    if !(1..index + 2).contains(&length) {
        return Err("Invalid length".to_string());
    }
    let shift = index + 1 - length;
    Ok((n >> shift) & stub)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bit_at() {
        assert_eq!(get_bit_at(0b0000_0001, 0).unwrap(), 1);
        assert_eq!(get_bit_at(0b0000_0010, 1).unwrap(), 1);
        assert_eq!(get_bit_at(0b0000_0100, 2).unwrap(), 1);
        assert_eq!(get_bit_at(0b0000_1000, 3).unwrap(), 1);
        assert_eq!(get_bit_at(0b0001_0000, 4).unwrap(), 1);
        assert_eq!(get_bit_at(0b0010_0000, 5).unwrap(), 1);
        assert_eq!(get_bit_at(0b0100_0000, 6).unwrap(), 1);
        assert_eq!(get_bit_at(0b1000_0000, 7).unwrap(), 1);
        assert_eq!(
            get_bit_at(0b0000_0000, 100),
            Err("Index out of bounds".to_string())
        );
    }

    #[test]
    fn test_get_num_at() {
        assert_eq!(get_num_at(0b0101_0001, 6, 2).unwrap(), 2);
        assert_eq!(get_num_at(0b0101_0001, 6, 3).unwrap(), 5);
        assert_eq!(
            get_num_at(0b0101_0001, 100, 3),
            Err("Offset out of bounds".to_string())
        );
        assert_eq!(
            get_num_at(0b0101_0001, 6, 100),
            Err("Invalid length".to_string())
        );
        assert_eq!(
            get_num_at(0b0101_0001, 1, 3),
            Err("Invalid length".to_string())
        );
    }
}
