
pub fn bit_is_set(bit: u8, input: u8) -> bool {
    (input & (1 << bit)) != 0
}

pub fn set_bit(bit: u8, input: u8) -> u8 {
    input | (1 << bit)
}

pub fn clear_bit(bit: u8, input: u8) -> u8 {
    input & !(1 << bit)
}

pub fn set_bit_from(bit: u8, input: u8, output_value: u8) -> u8 {
    match bit_is_set(bit, input) {
        true => set_bit(bit, output_value),
        false => clear_bit(bit, output_value)
    }
}

//fn set_bit_from(bit: u8, from: u8, operand: u8) -> u8 {
//    let mask = 1 << bit;
//    let bit_value = from & mask;
//    
//    match bit_value {
//        0 => { to & !mask }
//        _ => { to |  mask }
//    }
//}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_is_set() {
        let test_pattern = 0b00101110;

        assert!(!bit_is_set(0, test_pattern));
        assert!( bit_is_set(1, test_pattern));
        assert!( bit_is_set(2, test_pattern));
        assert!( bit_is_set(3, test_pattern));
        assert!(!bit_is_set(4, test_pattern));
        assert!( bit_is_set(5, test_pattern));
        assert!(!bit_is_set(6, test_pattern));
        assert!(!bit_is_set(7, test_pattern));
    }

    #[test]
    fn test_set_bit() {
        let test_pattern = 0b00000000;
        assert!(set_bit(0, test_pattern) == 1);
    }

    #[test]
    fn test_clear_bit() {
        let test_pattern = 0b01000000;
        assert!(clear_bit(6, test_pattern) == 0);
    }

    #[test]
    fn test_set_bit_from() {
        let input      = 0b01010101;
        let output_val = 0b11110001;

        assert!(set_bit_from(7, input, output_val) == 0b01110001);
        assert!(set_bit_from(6, input, output_val) == 0b11110001);
        assert!(set_bit_from(2, input, output_val) == 0b11110101);
    }
}