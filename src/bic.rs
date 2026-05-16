//! Sequential Binary Interpolative Coding (BIC) for strictly-positive integer sequences.
//!
//! Wire format: `[gamma(n)] [gamma(T[0])] [bsbin splits...]`
//!
//! Ported from `madeofbits/bic.go` (which implements Algorithm 1 of Moffat 2024).
//! All values must be strictly positive; zero values panic. The sum of all values
//! must fit in [`BicUint`].

/// Value type for BIC encoding. Switch to `u64` if larger sums are needed.
pub type BicUint = u32;

/// Encode `values` (all > 0) into a self-describing BIC bitstream.
pub fn encode(values: &[BicUint]) -> Vec<u8> {
    let n = values.len();
    if n == 0 {
        return Vec::new();
    }
    for (i, &v) in values.iter().enumerate() {
        assert_ne!(v, 0, "BIC encode: values[{i}] is zero; all values must be > 0");
    }

    // Implicit tree in a single buffer of length 2n-1: `t[0..n-1]` are internal nodes,
    // `t[n-1..2n-1]` are leaves. Build bottom-up so each internal node's value is
    // available before its parent is computed.
    let mut t = vec![0 as BicUint; 2 * n - 1];
    t[n - 1..].copy_from_slice(values);
    for p in (0..n - 1).rev() {
        t[p] = bic_combine(t[2 * p + 1], t[2 * p + 2]);
    }

    let mut w = BitWriter::new(16 + n as u32 * BicUint::BITS / 8);
    gamma_write(&mut w, n as BicUint);
    gamma_write(&mut w, t[0]);
    for p in 0..n - 1 {
        bsbin_write(&mut w, bic_mapping(t[2 * p + 1], t[2 * p + 2]), bic_range(t[p]));
    }
    w.finish()
}

/// Decode a BIC bitstream back into a `Vec<BicUint>`.
pub fn decode(data: &[u8]) -> Vec<BicUint> {
    if data.is_empty() {
        return Vec::new();
    }
    let mut r = BitReader::new(data);
    let n = gamma_read(&mut r) as usize;
    if n == 0 {
        return Vec::new();
    }
    // Same single-buffer layout as encode; the leaves slice becomes the return value.
    let mut t = vec![0 as BicUint; 2 * n - 1];
    t[0] = gamma_read(&mut r);
    for p in 0..n - 1 {
        let v = bsbin_read(&mut r, bic_range(t[p]));
        let (vl, vr) = bic_mapping_inv(v, t[p]);
        t[2 * p + 1] = vl;
        t[2 * p + 2] = vr;
    }
    t.split_off(n - 1)
}

// =====================================================================
// Helper functions (Table 2 of Moffat 2024).
// =====================================================================

fn bic_combine(vl: BicUint, vr: BicUint) -> BicUint {
    vl + vr - 1
}

fn bic_range(vp: BicUint) -> BicUint {
    vp
}

fn bic_mapping(vl: BicUint, vr: BicUint) -> BicUint {
    if vl <= vr { vr - vl + 1 } else { vl - vr }
}

fn bic_mapping_inv(v: BicUint, vp: BicUint) -> (BicUint, BicUint) {
    let vl = if (v + vp) % 2 == 0 {
        1 + (vp - v) / 2
    } else {
        (vp + v + 1) / 2
    };
    let vr = vp - vl + 1;
    (vl, vr)
}

/// `floor(log2(x))`, returns 0 for `x == 0`.
fn bic_ilog2(x: BicUint) -> u32 {
    if x == 0 { 0 } else { BicUint::BITS - x.leading_zeros() - 1 }
}

// =====================================================================
// Bottom-short minimal binary code, LSB-first.
// Encodes `v` in `[1, r]`.
// =====================================================================

fn bsbin_write(w: &mut BitWriter, v: BicUint, r: BicUint) {
    if r == 1 {
        return;
    }
    let k = bic_ilog2(r);
    let short_count = ((1 as BicUint) << (k + 1)) - r;
    if v <= short_count {
        w.write_bits((v - 1) as u64, k);
    } else {
        let long_index = v - short_count - 1;
        let low_bits = short_count + long_index / 2;
        let extra_bit = long_index % 2;
        w.write_bits((low_bits as u64) | ((extra_bit as u64) << k), k + 1);
    }
}

fn bsbin_read(r: &mut BitReader, bound: BicUint) -> BicUint {
    if bound == 1 {
        return 1;
    }
    let k = bic_ilog2(bound);
    let short_count = ((1 as BicUint) << (k + 1)) - bound;
    let c = r.read_bits(k) as BicUint;
    if c < short_count {
        return c + 1;
    }
    let extra_bit = r.read_bits(1) as BicUint;
    let long_index = 2 * (c - short_count) + extra_bit;
    short_count + long_index + 1
}

// =====================================================================
// Elias gamma code, LSB-first.
// =====================================================================

fn gamma_write(w: &mut BitWriter, x: BicUint) {
    let k = bic_ilog2(x);
    // Unary prefix: k one-bits + one zero-bit, written as low (k+1) bits of (1<<k)-1.
    w.write_bits((((1 as BicUint) << k) - 1) as u64, k + 1);
    if k > 0 {
        let payload = (x & (((1 as BicUint) << k) - 1)) as u64;
        w.write_bits(payload, k);
    }
}

fn gamma_read(r: &mut BitReader) -> BicUint {
    let mut k: u32 = 0;
    while r.read_bits(1) == 1 {
        k += 1;
    }
    if k == 0 {
        return 1;
    }
    (1 << k) | (r.read_bits(k) as BicUint)
}

// =====================================================================
// LSB-first bit I/O with 64-bit windows.
// =====================================================================

struct BitWriter {
    data: Vec<u8>,
    bit_pos: u32,
}

impl BitWriter {
    fn new(size: u32) -> Self {
        Self {
            data: vec![0u8; (size + 15) as usize],
            bit_pos: 0,
        }
    }

    fn write_bits(&mut self, value: u64, nbits: u32) {
        if nbits == 0 {
            return;
        }
        let byte_offset = (self.bit_pos / 8) as usize;
        let bit_offset = self.bit_pos % 8;
        let avail = 64 - bit_offset;
        // First window.
        let mut window = u64::from_le_bytes(
            self.data[byte_offset..byte_offset + 8].try_into().unwrap(),
        );
        window |= value << bit_offset;
        self.data[byte_offset..byte_offset + 8].copy_from_slice(&window.to_le_bytes());
        if nbits > avail {
            // Spill the high bits into the next 64-bit window.
            let byte_offset2 = byte_offset + 8;
            let mut window2 = u64::from_le_bytes(
                self.data[byte_offset2..byte_offset2 + 8].try_into().unwrap(),
            );
            window2 |= value >> avail;
            self.data[byte_offset2..byte_offset2 + 8].copy_from_slice(&window2.to_le_bytes());
        }
        self.bit_pos += nbits;
    }

    fn finish(self) -> Vec<u8> {
        let num_bytes = ((self.bit_pos + 7) / 8) as usize;
        let mut out = self.data;
        out.truncate(num_bytes);
        out
    }
}

struct BitReader {
    /// Owned copy of the input with 7 trailing pad bytes so that 64-bit window reads
    /// at any in-bounds bit position are safe. (Go does the same — borrowing would
    /// require risky bounds-juggling at the tail.)
    data: Vec<u8>,
    bit_pos: u32,
}

impl BitReader {
    fn new(data: &[u8]) -> Self {
        let mut padded = vec![0u8; data.len() + 8];
        padded[..data.len()].copy_from_slice(data);
        Self {
            data: padded,
            bit_pos: 0,
        }
    }

    fn read_bits(&mut self, nbits: u32) -> u64 {
        if nbits == 0 {
            return 0;
        }
        let byte_offset = (self.bit_pos / 8) as usize;
        let bit_offset = self.bit_pos % 8;
        let window = u64::from_le_bytes(
            self.data[byte_offset..byte_offset + 8].try_into().unwrap(),
        );
        let avail = 64 - bit_offset;
        if nbits <= avail {
            let mask = if nbits == 64 { !0u64 } else { (1u64 << nbits) - 1 };
            let value = (window >> bit_offset) & mask;
            self.bit_pos += nbits;
            return value;
        }
        let lo = window >> bit_offset;
        self.bit_pos += avail;
        let hi = self.read_bits(nbits - avail);
        lo | (hi << avail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(values: &[BicUint]) {
        let encoded = encode(values);
        let decoded = decode(&encoded);
        assert_eq!(decoded, values, "round-trip failed for {values:?}");
    }

    #[test]
    fn round_trip_empty() {
        round_trip(&[]);
    }

    #[test]
    fn round_trip_single() {
        for v in [1u32, 2, 100, 1_000_000] {
            round_trip(&[v]);
        }
    }

    #[test]
    fn round_trip_small() {
        round_trip(&[1, 1, 1, 1]);
        round_trip(&[1, 2, 3, 4]);
        round_trip(&[4, 3, 2, 1]);
        round_trip(&[5, 10, 15, 20]);
        round_trip(&[1, 1, 1, 1, 1, 1, 1, 1]);
    }

    #[test]
    #[should_panic(expected = "values[2] is zero")]
    fn panics_on_zero() {
        encode(&[1, 2, 0, 3]);
    }

    /// Exhaustive small-input round-trip: every Vec<u32> of length 1..=4 with values
    /// in 1..=5. Catches every off-by-one in the tree arithmetic.
    #[test]
    fn round_trip_exhaustive_small() {
        for len in 1..=4 {
            // Iterate the cartesian product 1..=5 ^ len.
            let mut value: u32 = 0;
            let max = 5u32.pow(len);
            while value < max {
                let mut buf: Vec<BicUint> = Vec::with_capacity(len as usize);
                let mut v = value;
                for _ in 0..len {
                    buf.push((v % 5) + 1);
                    v /= 5;
                }
                round_trip(&buf);
                value += 1;
            }
        }
    }

    /// Property test: arbitrary Vec<u32> with all values > 0 round-trips.
    #[test]
    fn prop_round_trip_arbitrary() {
        use arbtest::arbtest;
        arbtest(|u| {
            let len = u.int_in_range(0usize..=64)?;
            let mut values: Vec<BicUint> = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(u.int_in_range(1u32..=1000)?);
            }
            round_trip(&values);
            Ok(())
        });
    }

    /// Gamma code round-trip for every positive integer up to a small bound.
    #[test]
    fn gamma_round_trip() {
        for x in 1u32..=2048 {
            let mut w = BitWriter::new(16);
            gamma_write(&mut w, x);
            let encoded = w.finish();
            let mut r = BitReader::new(&encoded);
            assert_eq!(gamma_read(&mut r), x, "gamma round-trip for {x}");
        }
    }

    /// bsbin round-trip: for every (r, v) with r in 1..=64 and v in 1..=r.
    #[test]
    fn bsbin_round_trip() {
        for r in 1u32..=64 {
            for v in 1u32..=r {
                let mut w = BitWriter::new(16);
                bsbin_write(&mut w, v, r);
                let encoded = w.finish();
                let mut rd = BitReader::new(&encoded);
                assert_eq!(bsbin_read(&mut rd, r), v, "bsbin round-trip for (r={r}, v={v})");
            }
        }
    }
}
