
pub fn bit_is_set(bit: u8, input: u8) -> bool {
    (input & (1 << bit)) != 0
}

pub fn bit_is_set_u16(bit: u16, input: u16) -> bool {
    (input & (1u16 << bit)) != 0
}

pub fn set_bit(bit: u8, input: &mut u8) {
    *input |= 1 << bit;
}

pub fn clear_bit(bit: u8, input: &mut u8) {
    *input &= !(1 << bit);
}

pub fn set_bit_from(bit: u8, from: u8, output_value: &mut u8) {
    match bit_is_set(bit, from) {
        true => set_bit(bit, output_value),
        false => clear_bit(bit, output_value)
    };
}

//pub fn set_bits_from_mask(source: u8, mask: u8, dest: &mut u8) {
//    *dest = (*dest & !mask) | (source & mask)
//}

pub fn set_bits_from_mask_u16(source: u16, mask: u16, dest: &mut u16) {
    *dest = (*dest & !mask) | (source & mask)
}

pub fn same_page(addr1: u16, addr2: u16) -> bool {
    let addr1_page = addr1 / 256u16;
    let addr2_page = addr2 / 256u16;

    addr1_page == addr2_page
}

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
        let mut test_pattern = 0b00000000;
        set_bit(0, &mut test_pattern);
        assert!(test_pattern == 1);
    }

    #[test]
    fn test_clear_bit() {
        let mut test_pattern = 0b01000000;
        clear_bit(6, &mut test_pattern);
        assert!(test_pattern == 0);
    }

    #[test]
    fn test_set_bit_from() {
        let input      = 0b01010101;
        let mut output_val = 0b11110001;

        for i in 0..8 {
            set_bit_from(i, input, &mut output_val);
        }

        assert!(output_val == input);
    }

    #[test]
    fn test_set_bits_from_mask_u16() {
        let input: u16    = 0x01AB;
        let mut dest: u16 = 0xF0DD;
        let expected: u16 = 0xF1DD;

        set_bits_from_mask_u16(input, 0x0300, &mut dest);

        assert!(dest == expected);
    }

    #[test]
    fn test_same_page() {
        assert!(same_page(0x0000, 0x00ff));
        assert!(same_page(0xff00, 0xffff));
        assert!(!same_page(0xff00, 0xfeff));
        assert!(!same_page(0x0000, 0x0100));
    }
}