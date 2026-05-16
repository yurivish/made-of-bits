use crate::bits::one_mask;

/// Fixed-size buffer of fixed-width integers. Designed to be written once and read many times.
/// A newly constructed `IntBuf` has the specified length and all elements are initialized to zero.
/// Elements are added by pushing them, which fills from the lowest index up. In typical use,
/// the buffer is initialized and then exactly `length` elements are pushed.
///
/// Byte-addressable storage with 64-bit little-endian read/write windows. Bit widths up to 56 are
/// supported — a value of up to 56 bits, plus up to 7 bits of byte-alignment offset, fits in a
/// single 64-bit window. Storage is padded with 7 trailing bytes so tail reads never run off
/// the end. Ported from `madeofbits/intbuf.go`.
#[derive(Clone)]
pub struct IntBuf {
    data: Box<[u8]>,
    length: u32,
    bit_width: u32,
    low_bit_mask: u64,
    /// Cursor measured in bits.
    write_cursor: u32,
}

impl IntBuf {
    /// Maximum supported bit width per element. 56 leaves 7 bits of byte-alignment slack
    /// inside the 64-bit read/write window.
    pub(crate) const MAX_BIT_WIDTH: u32 = 56;

    pub(crate) fn new(length: u32, bit_width: u32) -> Self {
        assert!(
            bit_width <= Self::MAX_BIT_WIDTH,
            "bit_width {bit_width} exceeds IntBuf::MAX_BIT_WIDTH ({})",
            Self::MAX_BIT_WIDTH,
        );

        let length_in_bits = length as u64 * bit_width as u64;
        let num_bytes = length_in_bits.div_ceil(8);
        // 7 bytes of trailing padding so 64-bit window reads at any valid bit position are safe.
        let alloc = (num_bytes + 7) as usize;
        Self {
            data: vec![0u8; alloc].into(),
            length,
            bit_width,
            low_bit_mask: if bit_width == 0 { 0 } else { one_mask::<u64>(bit_width) },
            write_cursor: 0,
        }
    }

    /// Push a value into the IntBuf.
    /// Panics if there is no room or the value exceeds the bit width.
    /// As a special case, any number of zeros can be pushed when `bit_width == 0`.
    pub(crate) fn push(&mut self, value: u32) {
        if self.bit_width == 0 {
            assert_eq!(value, 0, "cannot push nonzero value with zero bit width");
            return;
        }
        assert!(
            value as u64 <= self.low_bit_mask,
            "value {value} exceeds bit_width {}",
            self.bit_width,
        );
        assert!(
            self.write_cursor < self.length * self.bit_width,
            "IntBuf is full"
        );

        let bit_pos = self.write_cursor;
        let byte_offset = (bit_pos / 8) as usize;
        let bit_offset = bit_pos % 8;

        // Read 64-bit window, OR in the value, write back.
        let mut w = u64::from_le_bytes(self.data[byte_offset..byte_offset + 8].try_into().unwrap());
        w |= (value as u64) << bit_offset;
        self.data[byte_offset..byte_offset + 8].copy_from_slice(&w.to_le_bytes());

        self.write_cursor += self.bit_width;
    }

    pub(crate) fn get(&self, index: u32) -> u32 {
        assert!(index < self.length, "IntBuf index out of bounds");
        if self.bit_width == 0 {
            return 0;
        }

        let bit_pos = index * self.bit_width;
        let byte_offset = (bit_pos / 8) as usize;
        let bit_offset = bit_pos % 8;

        let w = u64::from_le_bytes(self.data[byte_offset..byte_offset + 8].try_into().unwrap());
        ((w >> bit_offset) & self.low_bit_mask) as u32
    }
}

#[cfg(test)]
mod tests {
    use crate::catch_unwind;

    use super::*;

    #[test]
    fn test_intbuf() {
        // should disallow getting an element from an empty IntBuf
        catch_unwind(|| IntBuf::new(0, 0).get(0)).unwrap_err();

        // should throw on out-of-bounds indices
        catch_unwind(|| IntBuf::new(3, 7).get(10)).unwrap_err();

        // should return zero elements before anything is pushed
        let xs = IntBuf::new(3, 7);
        for i in 0..3 {
            assert_eq!(xs.get(i), 0);
        }

        // should allow writing and reading elements
        let tests: &[(u32, &[u32])] = &[
            (0, &[0, 0, 0, 0]),
            (1, &[1, 0, 1, 0]),
            (5, &[1, 0, 1, 0]),
            (32, &[10, 0, 31, u32::MAX]),
        ];

        for &(bit_width, values) in tests {
            let mut xs = IntBuf::new(values.len() as u32, bit_width);

            if bit_width < 32 {
                // test pushing a too-large value
                let mut xs = xs.clone();
                catch_unwind(move || xs.push(1 << bit_width)).unwrap_err();
            }

            for (i, &v) in values.iter().enumerate() {
                // test the value before writing
                assert_eq!(xs.get(i as u32), 0);
                // push the value
                xs.push(v);
                // test the value has been pushed
                assert_eq!(xs.get(i as u32), v);

                if bit_width < 32 {
                    let mut xs = xs.clone();
                    let too_large = 1 << bit_width;
                    catch_unwind(move || xs.push(too_large)).unwrap_err();
                }
            }

            // It should disallow pushing beyond the end, unless the bit width is zero.
            if bit_width > 0 {
                let mut xs = xs.clone();
                catch_unwind(move || xs.push(0)).unwrap_err();
            } else {
                xs.push(0);
            }
        }
    }

    /// Round-trip every (length, bit_width, values) combination at small sizes, plus
    /// a few edge widths near the 32-bit and 56-bit caps. Property-style coverage to
    /// catch off-by-ones in the 64-bit window arithmetic at byte boundaries.
    #[test]
    fn test_intbuf_round_trip_arbitrary() {
        use arbtest::arbtest;
        arbtest(|u| {
            // length: 0..=32; bit_width: 0..=56 (the cap).
            let bit_width = u.int_in_range(0u32..=56)?;
            let length = u.int_in_range(0u32..=32)?;
            let mut buf = IntBuf::new(length, bit_width);
            let mask: u64 = if bit_width == 0 {
                0
            } else if bit_width >= 32 {
                u32::MAX as u64
            } else {
                (1u64 << bit_width) - 1
            };
            let mut values = Vec::with_capacity(length as usize);
            for _ in 0..length {
                // Generate a value within the bit-width (capped at u32::MAX since push takes u32).
                let raw = u.arbitrary::<u32>()? as u64;
                let v = (raw & mask) as u32;
                values.push(v);
                buf.push(v);
            }
            for (i, &v) in values.iter().enumerate() {
                assert_eq!(buf.get(i as u32), v, "round-trip at i={i}, bw={bit_width}");
            }
            Ok(())
        });
    }

    /// Push a 56-bit value (the maximum width). u32 input means we can't actually
    /// store >32-bit values yet, but we *can* verify the constructor and round-trip
    /// at the boundary widths.
    #[test]
    fn test_intbuf_max_width() {
        for bw in [31u32, 32, 56] {
            let mut buf = IntBuf::new(4, bw);
            let mask: u32 = if bw >= 32 {
                u32::MAX
            } else {
                (1u32 << bw) - 1
            };
            buf.push(mask);
            buf.push(0);
            buf.push(1);
            buf.push(mask);
            assert_eq!(buf.get(0), mask);
            assert_eq!(buf.get(1), 0);
            assert_eq!(buf.get(2), 1);
            assert_eq!(buf.get(3), mask);
        }
    }

    /// Constructing with bit_width > 56 panics.
    #[test]
    fn test_intbuf_rejects_wide_bit_widths() {
        catch_unwind(|| {
            IntBuf::new(4, 57);
        })
        .unwrap_err();
        catch_unwind(|| {
            IntBuf::new(4, 64);
        })
        .unwrap_err();
    }
}
