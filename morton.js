// Morton codes
//
// JavaScript bitwise operations work on 32-bit integers, so we can use the functions
// from [this blog post](https://fgiesen.wordpress.com/2009/12/13/decoding-morton-codes/). 
// See the comments there for a good explanation of how these work.
//
// We can encode up to 16-bit codes 2d and 10-bit codes in 3d. Note that the order is 
// like a reflected Z, traversed from bottom left to top right : *bl*, *br*, *tl*, *tr*.

function encode2(x, y) { 
  return ((part1By1(y) << 1) + part1By1(x)) >>> 0;
}

function encode3(x, y, z) { 
  return ((part1By2(z) << 2) + (part1By2(y) << 1) + part1By2(x)) >>> 0;; 
}


function decode2x(code) { 
  return compact1By1(code >> 0);
}

function decode2y(code) { 
  return compact1By1(code >> 1);
}

// convenience function
function decode2(d) { 
  return [decode2x(d), decode2y(d)];
}

function decode3x(code) { 
  return compact1By2(code >> 0);
}

function decode3y(code) { 
  return compact1By2(code >> 1);
}

function decode3z(code) { 
  return compact1By2(code >> 2);
}

// Experimenting with byte interleaving since it is possible to implement using
// wasm simd128 (swizzle, shuffle)
function part8By8(x) {
  x &= 0x0000ffff; // x = ---- ---- ---- ---- fedc ba98 7654 3210
  x = (x ^ (x << 8)) & 0x00ff00ff; // x = ---- ---- fedc ba98 ---- ---- 7654 3210
  return x;
}

// "Insert" a 0 bit after each of the 16 low bits of x
function part1By1(x) {
  x &= 0x0000ffff; // x = ---- ---- ---- ---- fedc ba98 7654 3210
  x = (x ^ (x << 8)) & 0x00ff00ff; // x = ---- ---- fedc ba98 ---- ---- 7654 3210
  x = (x ^ (x << 4)) & 0x0f0f0f0f; // x = ---- fedc ---- ba98 ---- 7654 ---- 3210
  x = (x ^ (x << 2)) & 0x33333333; // x = --fe --dc --ba --98 --76 --54 --32 --10
  x = (x ^ (x << 1)) & 0x55555555; // x = -f-e -d-c -b-a -9-8 -7-6 -5-4 -3-2 -1-0
  return x;
}

function compact1By1(x) {
  x &= 0x55555555; // x = -f-e -d-c -b-a -9-8 -7-6 -5-4 -3-2 -1-0
  x = (x ^ (x >> 1)) & 0x33333333; // x = --fe --dc --ba --98 --76 --54 --32 --10
  x = (x ^ (x >> 2)) & 0x0f0f0f0f; // x = ---- fedc ---- ba98 ---- 7654 ---- 3210
  x = (x ^ (x >> 4)) & 0x00ff00ff; // x = ---- ---- fedc ba98 ---- ---- 7654 3210
  x = (x ^ (x >> 8)) & 0x0000ffff; // x = ---- ---- ---- ---- fedc ba98 7654 3210
  return x;
}

// "Insert" two 0 bits after each of the 10 low bits of x
function part1By2(x) {
  x &= 0x000003ff; // x = ---- ---- ---- ---- ---- --98 7654 3210
  x = (x ^ (x << 16)) & 0xff0000ff; // x = ---- --98 ---- ---- ---- ---- 7654 3210
  x = (x ^ (x << 8)) & 0x0300f00f; // x = ---- --98 ---- ---- 7654 ---- ---- 3210
  x = (x ^ (x << 4)) & 0x030c30c3; // x = ---- --98 ---- 76-- --54 ---- 32-- --10
  x = (x ^ (x << 2)) & 0x09249249; // x = ---- 9--8 --7- -6-- 5--4 --3- -2-- 1--0
  return x;
}

function compact1By2(x) {
  x &= 0x09249249; // x = ---- 9--8 --7- -6-- 5--4 --3- -2-- 1--0
  x = (x ^ (x >> 2)) & 0x030c30c3; // x = ---- --98 ---- 76-- --54 ---- 32-- --10
  x = (x ^ (x >> 4)) & 0x0300f00f; // x = ---- --98 ---- ---- 7654 ---- ---- 3210
  x = (x ^ (x >> 8)) & 0xff0000ff; // x = ---- --98 ---- ---- ---- ---- 7654 3210
  x = (x ^ (x >> 16)) & 0x000003ff; // x = ---- ---- ---- ---- ---- --98 7654 3210
  return x;
}

// From https://twitter.com/jonahharris/status/1337087177591820290/photo/1
// Used with permission from Jonah, who can't remember where he got it but
// says he obtained it under the BSD license.
// The basic idea is to determine the MSB, then split the range below that,
// using the common prefix together with  a calculation for the new y/x
// positions indicating the split point.
// See also: https://snorrwe.onrender.com/posts/morton-table/#range-query-splitting
function litMaxBigMin(uMin, uMax) {
  const xor = uMin ^ uMax;
  const uMSBD = 1 << (31 - Math.clz32(xor)); // note: fails for xor = 0 (31-clz is negative)
  const xMask = 0x55555555;
  const yMask = 0xaaaaaaaa; //~xMask;
  const splitXAxis = uMSBD & xMask;
  const splitMask = splitXAxis ? xMask : yMask;
  const uMSMask = (uMSBD - 1) & splitMask;
  const uLSMask = (uMSBD - 1) & ~splitMask;
  const uBSCommon = uMin & ~(uMSBD + uMSBD - 1);
  const uLitMax = uBSCommon | uMSMask | (uLSMask & uMax);
  const uBigMin = uBSCommon | uMSBD | (uLSMask & uMin);
  return { litMax: uLitMax >>> 0, bigMin: uBigMin >>> 0 };
}

