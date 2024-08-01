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
}
