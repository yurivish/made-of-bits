// Translated to Rust from C code provided by Fabian Geisen:
// https://fgiesen.wordpress.com/2009/12/13/decoding-morton-codes/
// Note: We can implement 4d codes by performing two 2d interleavings.
// Note: This encodes 2d as yxyxyxyx and 3d as zyxzyxzyxzyx.

// Note: `tl` means top_left, and `br` means bottom_right
// in a right-side-up Z-pattern with (0, 0) at the top left:
//
// tl tr
// bl br

use std::{ffi::CStr, ops::RangeInclusive};

const X_MASK_2D: u32 = 0b01010101010101010101010101010101;
const Y_MASK_2D: u32 = !X_MASK_2D;

// 3D dimension masks. In a 3D Morton code each dimension's bits occupy every 3rd
// position; the masks below pick those bits out.
const X_MASK_3D: u32 = 0b01001001001001001001001001001001;
const Y_MASK_3D: u32 = 0b10010010010010010010010010010010;
const Z_MASK_3D: u32 = 0b00100100100100100100100100100100;

const DIM_MASKS_2D: [u32; 2] = [X_MASK_2D, Y_MASK_2D];
const DIM_MASKS_3D: [u32; 3] = [X_MASK_3D, Y_MASK_3D, Z_MASK_3D];

/// Mask selecting every bit position belonging to dimension `dim` (0-indexed) in an
/// `ndims`-dimensional Morton code.
///
/// Fast path for 2D and 3D (the only widths Morton encoding currently supports); for
/// other widths, computes the mask via a loop.
pub fn dim_mask(dim: usize, ndims: usize) -> u32 {
    if ndims == 2 {
        return DIM_MASKS_2D[dim];
    }
    if ndims == 3 {
        return DIM_MASKS_3D[dim];
    }
    let mut mask = 0u32;
    let mut i = dim as u32;
    while i < 32 {
        mask |= 1u32 << i;
        i += ndims as u32;
    }
    mask
}

/// Set `bit` in `val` and clear every lower bit belonging to the same dimension
/// (identified by `dm`). Tropf's `LOAD_xxx10000` for interleaved Morton codes.
fn load_xxx_10000(mut val: u32, bit: u32, dm: u32) -> u32 {
    val |= bit;
    val &= !((bit - 1) & dm);
    val
}

/// Clear `bit` in `val` and set every lower bit belonging to the same dimension.
/// Tropf's `LOAD_xxx01111`.
fn load_xxx_01111(mut val: u32, bit: u32, dm: u32) -> u32 {
    val &= !bit;
    val |= (bit - 1) & dm;
    val
}

/// Largest Morton code in the search rectangle `[min, max]` whose Z-value is strictly
/// less than `div`. Precondition: `Z(min) < Z(div) < Z(max)`.
///
/// Implements Tropf's LITMAX algorithm for pre-interleaved Morton codes (Tropf & Herzog,
/// "Multidimensional Range Search in Dynamically Balanced Trees", Angewandte Informatik,
/// Feb 1981). Useful as a primitive for N-dimensional range queries on Morton-encoded
/// data — e.g., the future `morton_count_batch` operation.
pub fn litmax(min: u32, max: u32, div: u32, ndims: usize) -> u32 {
    let mut litmax = max;
    let mut min = min;
    let mut max = max;
    let mut masks = [0u32; 8];
    for d in 0..ndims {
        masks[d] = dim_mask(d, ndims);
    }
    for pos in (0..32).rev() {
        let bit = 1u32 << pos;
        let dm = masks[(pos as usize) % ndims];

        let div_bit = (div & bit) != 0;
        let min_bit = (min & bit) != 0;
        let max_bit = (max & bit) != 0;

        match (div_bit, min_bit, max_bit) {
            (false, false, false) => {}                      // same section, continue
            (false, false, true) => max = load_xxx_01111(max, bit, dm), // shrink max to lower section
            (false, true, true) => return litmax,            // div below range, done
            (true, false, false) => return max,              // div above range, litmax is max
            (true, false, true) => {
                litmax = load_xxx_01111(max, bit, dm);
                min = load_xxx_10000(min, bit, dm);
            }
            (true, true, true) => {}                         // same section, continue
            // (false, true, false): max precedes min in Morton order — invalid precondition.
            // (true, true, false): same. Treat as no-op to match Go.
            _ => {}
        }
    }
    litmax
}

/// Smallest Morton code in the search rectangle `[min, max]` whose Z-value is strictly
/// greater than `div`. Precondition: `Z(min) < Z(div) < Z(max)`.
///
/// Tropf's BIGMIN algorithm. See [`litmax`] for details.
pub fn bigmin(min: u32, max: u32, div: u32, ndims: usize) -> u32 {
    let mut bigmin = min;
    let mut min = min;
    let mut max = max;
    let mut masks = [0u32; 8];
    for d in 0..ndims {
        masks[d] = dim_mask(d, ndims);
    }
    for pos in (0..32).rev() {
        let bit = 1u32 << pos;
        let dm = masks[(pos as usize) % ndims];

        let div_bit = (div & bit) != 0;
        let min_bit = (min & bit) != 0;
        let max_bit = (max & bit) != 0;

        match (div_bit, min_bit, max_bit) {
            (false, false, false) => {}                       // continue
            (false, false, true) => {
                bigmin = load_xxx_10000(min, bit, dm);
                max = load_xxx_01111(max, bit, dm);
            }
            (false, true, true) => return min,                // div below range, bigmin is min
            (true, false, false) => return bigmin,            // div above range, done
            (true, false, true) => min = load_xxx_10000(min, bit, dm), // advance min
            (true, true, true) => {}                          // continue
            _ => {}
        }
    }
    bigmin
}

type CStringResult<T> = Result<T, &'static CStr>;

fn well_formed_bbox(tl: u32, br: u32) -> CStringResult<()> {
    // Assert that the height and width are both nonnegative.
    if (br & X_MASK_2D < tl & X_MASK_2D) || (br & Y_MASK_2D < tl & Y_MASK_2D) {
        Err(c"bottom-right may not precede top-left in either the x or y dimension")
    } else {
        Ok(())
    }
}

// note: assumes the bounding box is valid
fn bbox(tl: u32, br: u32) -> CStringResult<(u32, u32)> {
    // consider moving this check elsewhere; this function is called many times per split_bbox_2d_impl invocation
    // but we really only need to check the initial range.
    well_formed_bbox(tl, br)?;

    // Width and height are inclusive (if tl === br, then the width and height are both 1)
    let width = decode2x(br) - decode2x(tl) + 1;
    let height = decode2y(br) - decode2y(tl) + 1;
    Ok((width, height))
}

/// Checks whether the code range tl..br is fully contained inside its 2d bounding box.
/// Works by checking whether the number of codes in the code range (br - tl) does not
/// exceed the number of codes in the rectangular bounding box.
/// This relies on the fact that the region tl..br always at covers the bounding box,
/// and may cover more area than that .
fn range_contained_in_bbox_2d(tl: u32, br: u32) -> CStringResult<bool> {
    bbox(tl, br).map(|(width, height)| {
        let count = width * height;
        br - tl < count
    })
}

// From https://twitter.com/jonahharris/status/1337087177591820290/photo/1
// Used with permission from Jonah, who can't remember where he got it but
// says he obtained it under the BSD license.
// The basic idea is to determine the MSB, then split the range below that,
// using the common prefix together with a calculation for the new x/y
// positions indicating the split point.
// See also: https://snorrwe.onrender.com/posts/morton-table/#range-query-splitting
// Ported from https://observablehq.com/@yurivish/morton-codes#litMaxBigMin
pub fn litmax_bigmin_2d(min: u32, max: u32) -> (u32, u32) {
    // in order to not throw errors, (since we don't have a way to return Result<(u32, u32)>)
    // swap the guys...
    if min == max {
        return (min, max);
    }
    let (min, max) = (min.min(max), min.max(max));
    let diff = min ^ max;
    let diff_msb = 1 << (31 - diff.leading_zeros());
    let split_x = diff_msb & X_MASK_2D > 0;
    let split_mask = if split_x { X_MASK_2D } else { Y_MASK_2D };
    // mask for bits below the split point along the major dimension
    let major_mask = (diff_msb - 1) & split_mask;
    // mask for bits below the split point along the minor dimension
    let minor_mask = (diff_msb - 1) & !split_mask;
    // common prefix
    let common = min & !(diff_msb + (diff_msb - 1));
    // take the minimum value, but set its minor coordinate to the maximum value or something similar...
    // (minx, miny) -> (minx->splitpoint, maxy)
    // this really would be nice to write a little explainer about - i always get confused about these z order things
    let litmax = common | major_mask | (minor_mask & max); // if x is the major dimension, set all x values to 1 and take the y values from the max
    let bigmin = common | diff_msb | (minor_mask & min); // if x is the major dim, set the y high bit to 1 and the rest to 0
    (litmax, bigmin)
}

// Other things we could do:
// Apply a minimum length before splitting condition, or percentage out of bounds type thing,
// as makes sense for the problem at hand.
pub fn split_bbox_2d(tl: u32, br: u32) -> CStringResult<Box<[u32]>> {
    // stack of intervals to process, which will be recursively split if they are not contained in their bbox
    let mut stack = vec![(tl, br)];
    let mut ret = vec![];
    while let Some((lo, hi)) = stack.pop() {
        // if the range under consideration is contained within its bounding box, emit it.
        // note: since we always keep one endpoint from previously (see the other branch),
        // this range check will wastefully decode the same morton codes again and again.
        // we can probably do better...
        if range_contained_in_bbox_2d(lo, hi)? {
            // if we can extend the most recently emitted range, do so in-place.
            if let Some(last_hi) = ret.last_mut() {
                if *last_hi + 1 == lo {
                    *last_hi = hi;
                    continue;
                }
            }
            // otherwise, emit the range.
            ret.push(lo);
            ret.push(hi);
        } else {
            // otherwise, split the range, taking care to order the stack elements such that we emit ranges in ascending order.
            let (litmax, bigmin) = litmax_bigmin_2d(lo, hi);
            stack.push((bigmin, hi));
            stack.push((lo, litmax));
        }
    }
    Ok(ret.into())
}

// pub fn split_bbox_2d(tl: u32, br: u32) -> CStringResult<Vec<RangeInclusive<u32>>> {
//     // stack of intervals to process, which will be recursively split if they are not contained in their bbox
//     let mut stack = vec![(tl, br)];
//     let mut ret = vec![];
//     while let Some((lo, hi)) = stack.pop() {
//         // if the range under consideration is contained within its bounding box, emit it.
//         // note: since we always keep one endpoint from previously (see the other branch),
//         // this range check will wastefully decode the same morton codes again and again.
//         // we can probably do better...
//         if range_contained_in_bbox_2d(lo, hi)? {
//             // if we can extend the most recently emitted range, do so in-place.
//             if let Some(last_hi) = ret.last_mut() {
//                 if *last_hi + 1 == lo {
//                     *last_hi = hi;
//                     continue;
//                 }
//             }
//             // otherwise, emit the range.
//             ret.push(lo);
//             ret.push(hi);
//         } else {
//             // otherwise, split the range, taking care to order the stack elements such that we emit ranges in ascending order.
//             let (litmax, bigmin) = litmax_bigmin_2d(lo, hi);
//             stack.push((bigmin, hi));
//             stack.push((lo, litmax));
//         }
//     }
//     Ok(ret.chunks_exact(2).map(|x| x[0]..=x[1]).collect())
// }

pub const fn encode2(x: u32, y: u32) -> u32 {
    (part_1_by_1(y) << 1) + part_1_by_1(x)
    // if (x >= 1 << 16) || (y >= 1 << 16) {
    //     Err(c"x and y must each be less than 2^16 for the encoded value to fit into 32 bits")
    // } else {
    //     Ok((part_1_by_1(y) << 1) + part_1_by_1(x))
    // }
}

pub const fn encode3(x: u32, y: u32, z: u32) -> u32 {
    (part_1_by_2(z) << 2) + (part_1_by_2(y) << 1) + part_1_by_2(x)
    // if (x > 1 << 11) || (y > 1 << 11) || (z > 1 << 10) {
    //     Err(c"x and y must each be less than 2^11 and z must be less than 2^10 for the encoded value to fit into 32 bits")
    // } else {
    //     Ok((part_1_by_2(z) << 2) + (part_1_by_2(y) << 1) + part_1_by_2(x))
    // }
}

pub const fn decode2x(code: u32) -> u32 {
    compact_1_by_1(code)
}

pub const fn decode2y(code: u32) -> u32 {
    compact_1_by_1(code >> 1)
}

pub const fn decode3x(code: u32) -> u32 {
    compact_1_by_2(code)
}

pub const fn decode3y(code: u32) -> u32 {
    compact_1_by_2(code >> 1)
}

pub const fn decode3z(code: u32) -> u32 {
    compact_1_by_2(code >> 2)
}

// "Insert" a 0 bit after each of the 16 low bits of x
const fn part_1_by_1(x: u32) -> u32 {
    let mut x = x;
    x &= 0x0000ffff; //                 x = ---- ---- ---- ---- fedc ba98 7654 3210
    x = (x ^ (x << 8)) & 0x00ff00ff; // x = ---- ---- fedc ba98 ---- ---- 7654 3210
    x = (x ^ (x << 4)) & 0x0f0f0f0f; // x = ---- fedc ---- ba98 ---- 7654 ---- 3210
    x = (x ^ (x << 2)) & 0x33333333; // x = --fe --dc --ba --98 --76 --54 --32 --10
    x = (x ^ (x << 1)) & 0x55555555; // x = -f-e -d-c -b-a -9-8 -7-6 -5-4 -3-2 -1-0
    x
}

// "Insert" two 0 bits after each of the 10 low bits of x
const fn part_1_by_2(x: u32) -> u32 {
    let mut x = x;
    x &= 0x000003ff; //                  x = ---- ---- ---- ---- ---- --98 7654 3210
    x = (x ^ (x << 16)) & 0xff0000ff; // x = ---- --98 ---- ---- ---- ---- 7654 3210
    x = (x ^ (x << 8)) & 0x0300f00f; //  x = ---- --98 ---- ---- 7654 ---- ---- 3210
    x = (x ^ (x << 4)) & 0x030c30c3; //  x = ---- --98 ---- 76-- --54 ---- 32-- --10
    x = (x ^ (x << 2)) & 0x09249249; //  x = ---- 9--8 --7- -6-- 5--4 --3- -2-- 1--0
    x
}

// Inverse of part_1_by_1 - "delete" all odd-indexed bits
const fn compact_1_by_1(x: u32) -> u32 {
    let mut x = x;
    x &= 0x55555555; //                 x = -f-e -d-c -b-a -9-8 -7-6 -5-4 -3-2 -1-0
    x = (x ^ (x >> 1)) & 0x33333333; // x = --fe --dc --ba --98 --76 --54 --32 --10
    x = (x ^ (x >> 2)) & 0x0f0f0f0f; // x = ---- fedc ---- ba98 ---- 7654 ---- 3210
    x = (x ^ (x >> 4)) & 0x00ff00ff; // x = ---- ---- fedc ba98 ---- ---- 7654 3210
    x = (x ^ (x >> 8)) & 0x0000ffff; // x = ---- ---- ---- ---- fedc ba98 7654 3210
    x
}

// Inverse of part_1_by_2 - "delete" all bits not at positions divisible by 3
const fn compact_1_by_2(x: u32) -> u32 {
    let mut x = x;
    x &= 0x09249249; //                  x = ---- 9--8 --7- -6-- 5--4 --3- -2-- 1--0
    x = (x ^ (x >> 2)) & 0x030c30c3; //  x = ---- --98 ---- 76-- --54 ---- 32-- --10
    x = (x ^ (x >> 4)) & 0x0300f00f; //  x = ---- --98 ---- ---- 7654 ---- ---- 3210
    x = (x ^ (x >> 8)) & 0xff0000ff; //  x = ---- --98 ---- ---- ---- ---- 7654 3210
    x = (x ^ (x >> 16)) & 0x000003ff; // x = ---- ---- ---- ---- ---- --98 7654 3210
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = litmax_bigmin_2d(123, 456);
        assert_eq!(result, (221, 298));
    }

    #[test]
    fn wat() {
        litmax_bigmin_2d(3, 3);

        let result = split_bbox_2d(3, 48);
        assert_eq!(
            result,
            Ok(
                vec![3, 3, 6, 7, 9, 9, 11, 15, 18, 18, 24, 24, 26, 26, 33, 33, 36, 37, 48, 48]
                    .into()
            )
        );

        assert!(range_contained_in_bbox_2d(3, 4).is_err());
        assert_eq!(range_contained_in_bbox_2d(2, 3), Ok(true));
    }

    /// 3D masks are disjoint pairwise and union to all 32 bits.
    #[test]
    fn test_3d_masks_partition() {
        assert_eq!(X_MASK_3D & Y_MASK_3D, 0);
        assert_eq!(X_MASK_3D & Z_MASK_3D, 0);
        assert_eq!(Y_MASK_3D & Z_MASK_3D, 0);
        assert_eq!(X_MASK_3D | Y_MASK_3D | Z_MASK_3D, u32::MAX);
    }

    /// `dim_mask` agrees with the constants for 2D and 3D.
    #[test]
    fn test_dim_mask_consistency() {
        assert_eq!(dim_mask(0, 2), X_MASK_2D);
        assert_eq!(dim_mask(1, 2), Y_MASK_2D);
        assert_eq!(dim_mask(0, 3), X_MASK_3D);
        assert_eq!(dim_mask(1, 3), Y_MASK_3D);
        assert_eq!(dim_mask(2, 3), Z_MASK_3D);
        // Generic fallback for ndims = 4.
        assert_eq!(dim_mask(0, 4), 0b00010001000100010001000100010001);
    }

    /// 2D encode/decode round-trip on small values.
    #[test]
    fn test_2d_encode_decode_roundtrip() {
        for x in 0..16u32 {
            for y in 0..16u32 {
                let code = encode2(x, y);
                assert_eq!(decode2x(code), x, "decode2x failed for ({x}, {y})");
                assert_eq!(decode2y(code), y, "decode2y failed for ({x}, {y})");
            }
        }
    }

    /// 3D encode/decode round-trip on small values.
    #[test]
    fn test_3d_encode_decode_roundtrip() {
        for x in 0..8u32 {
            for y in 0..8u32 {
                for z in 0..8u32 {
                    let code = encode3(x, y, z);
                    assert_eq!(decode3x(code), x, "decode3x failed for ({x},{y},{z})");
                    assert_eq!(decode3y(code), y, "decode3y failed for ({x},{y},{z})");
                    assert_eq!(decode3z(code), z, "decode3z failed for ({x},{y},{z})");
                }
            }
        }
    }

    /// `litmax(min, max, div, 2)` agrees with `litmax_bigmin_2d(min, max).0` when
    /// `Z(min) < Z(div) < Z(max)` and `div` is between min and max along the search.
    /// The simpler `litmax_bigmin_2d` doesn't take a div but solves the related problem
    /// of splitting at the most-significant-differing-bit; matching values fall out for
    /// the exact-MSB-split case.
    /// `litmax(min, max, div, 2)` returns a value strictly less than `div`,
    /// given the Tropf precondition that `min` and `max` are corners of a valid 2D bbox
    /// and `min < div < max`. (Outside that precondition the algorithm may produce
    /// arbitrary values — Tropf's contract is conditional.)
    #[test]
    fn test_litmax_within_range() {
        for min in 0u32..32 {
            for max in (min + 2)..64 {
                if !well_formed_bbox(min, max).is_ok() {
                    continue;
                }
                for div in (min + 1)..max {
                    let l = litmax(min, max, div, 2);
                    assert!(l < div, "litmax({min},{max},{div}) = {l} not < div");
                }
            }
        }
    }

    /// Symmetric check for `bigmin`.
    #[test]
    fn test_bigmin_within_range() {
        for min in 0u32..32 {
            for max in (min + 2)..64 {
                if !well_formed_bbox(min, max).is_ok() {
                    continue;
                }
                for div in (min + 1)..max {
                    let b = bigmin(min, max, div, 2);
                    assert!(b > div, "bigmin({min},{max},{div}) = {b} not > div");
                }
            }
        }
    }
}
