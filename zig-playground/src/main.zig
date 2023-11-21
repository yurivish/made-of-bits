const std = @import("std");
const builtin = @import("builtin");
const Allocator = std.mem.Allocator;
const DynamicBitSet = std.DynamicBitSet;
const ArrayList = std.ArrayList;

const BlockInt = DynamicBitSet.MaskInt; // todo: figure out where we wanna use this
const BlockBits = @bitSizeOf(BlockInt); // this must be a power of 2 (todo: comptime assert it)

const ShiftInt = DynamicBitSet.ShiftInt;
const ShiftBits = @bitSizeOf(ShiftInt);

const assert = std.debug.assert;
const testing = std.testing;

// todo: Should we have error returns from WASM endpoints via a parameter
// that takes a string literal or null, i.e. 0 pointer?
// a [*]const u8, length u8 sort of thing...
// or maybe better a zero-sentinel-terminated

// export fn add(a: i32, b: i32) i32 {
//     const message = "happy happy joy joy";
//     consoleLog(message, message.len);
//     return a + b;
// }

// extern fn consoleLog(message: [*]const u8, length: u8) void;

var global_alloc: Allocator = undefined;

export fn init() void {
    global_alloc = comptime if (builtin.cpu.arch.isWasm())
        std.heap.wasm_allocator
    else blk: {
        // this allocatorhas requirements on i/o from the os module
        // that fail to compile on wasm, so we only use it on other architectures
        var gpa = std.heap.GeneralPurposeAllocator(.{}){};
        break :blk gpa.allocator();
    };
}

export fn DenseBitVecBuilder_init(len: usize) *DenseBitVecBuilder {
    var bv = global_alloc.create(DenseBitVecBuilder) catch @panic("allocation failed");
    bv.* = DenseBitVecBuilder.init(global_alloc, len) catch @panic("allocation failed");
    return bv;
}

export fn DenseBitVecBuilder_one(self: *DenseBitVecBuilder, index: usize, count: usize) void {
    self.one(index, count);
}

export fn DenseBitVecBuilder_build(self: *DenseBitVecBuilder, rank_sr: u32, select0_sr: u32, select1_sr: u32) *DenseBitVec {
    var bv = global_alloc.create(DenseBitVec) catch @panic("allocation failed");
    bv.* = self.build(global_alloc, Config{
        .rank_sr = @intCast(rank_sr),
        .select0_sr = @intCast(select0_sr),
        .select1_sr = @intCast(select1_sr),
    }) catch @panic("could not construct DenseBitVec");
    return bv;
}

export fn DenseBitVec_universe_size(self: *DenseBitVec) usize {
    return self.universe_size();
}

export fn DenseBitVec_num_ones(self: *DenseBitVec) usize {
    return self.num_ones();
}

export fn DenseBitVec_num_zeros(self: *DenseBitVec) usize {
    return self.num_zeros();
}

export fn DenseBitVec_rank1(self: *DenseBitVec, i: usize) usize {
    return self.rank1(i);
}

export fn DenseBitVec_rank0(self: *DenseBitVec, i: usize) usize {
    return self.rank0(i);
}

export fn DenseBitVec_select1(self: *DenseBitVec, n: usize) usize {
    return self.select1(n).?;
}

export fn DenseBitVec_select0(self: *DenseBitVec, n: usize) usize {
    return self.select0(n).?;
}

const Config = struct {
    rank_sr: ShiftInt = 10, // each sample counts the number of 1-bits
    select0_sr: ShiftInt = 10,
    select1_sr: ShiftInt = 10,
};

const DenseBitVecBuilder = struct {
    const Self = @This();

    bits: DynamicBitSet,

    pub fn init(alloc: Allocator, len: usize) !Self {
        return .{ .bits = try DynamicBitSet.initEmpty(alloc, len) };
    }

    pub fn one(self: *Self, index: usize, count: usize) void {
        assert(count == 1);
        self.bits.set(index);
    }

    pub fn build(self: Self, alloc: Allocator, config: Config) !DenseBitVec {
        return try DenseBitVec.init(alloc, self.bits, config);
    }
};

const DenseBitVec = struct {
    const Self = @This();

    bits: DynamicBitSet,
    config: Config,
    r: ArrayList(usize),
    s1: ArrayList(usize),
    s0: ArrayList(usize),
    ones_count: usize,

    pub fn init(alloc: Allocator, bits: DynamicBitSet, config: Config) !Self {
        var r = ArrayList(usize).init(alloc);
        var s1 = ArrayList(usize).init(alloc);
        var s0 = ArrayList(usize).init(alloc);

        const rank_sr_blocks = (@as(BlockInt, 1) << config.rank_sr); // >> ShiftBits;
        const select1_sr = @as(BlockInt, 1) << config.select1_sr;
        const select0_sr = @as(BlockInt, 1) << config.select0_sr;

        var preceding_ones: usize = 0;
        var zeros_threshold: usize = 0;
        var ones_threshold: usize = 0;

        const blocks = bitmasks(bits);
        for (blocks, 0..) |block, i| {
            const preceding_bits = i << ShiftBits;
            const preceding_zeros = preceding_bits - preceding_ones;

            const block_ones = @popCount(block);
            const block_zeros = if (i < blocks.len - 1)
                BlockBits - block_ones
            else
                (bits.capacity() % BlockBits) - block_ones;

            if (i % rank_sr_blocks == 0)
                try r.append(preceding_ones);

            if (preceding_ones + block_ones > ones_threshold) {
                const correction = ones_threshold - preceding_ones;
                try s1.append(preceding_bits | correction);
                ones_threshold += select1_sr;
                assert((preceding_bits & correction) == 0);
            }

            if (preceding_zeros + block_zeros > zeros_threshold) {
                const correction = zeros_threshold - preceding_ones;
                try s0.append(preceding_bits | correction);
                zeros_threshold += select0_sr;
                assert((preceding_bits & correction) == 0);
            }

            preceding_ones += block_ones;
        }

        return Self{
            .bits = bits,
            .config = config,
            .r = r,
            .s0 = s0,
            .s1 = s1,
            .ones_count = preceding_ones,
        };
    }

    pub fn rank1(self: Self, i: usize) usize {
        if (i >= self.universe_size()) return self.ones_count;

        // index of the rank sample containing the `i`-th bit
        const i_rank = i >> self.config.rank_sr;

        // index of the block pointed to by the rank sample
        var i_start = i_rank << (self.config.rank_sr - ShiftBits);

        // index of the block containing the `i`-th bit
        const i_end = i >> ShiftBits;

        var count = self.r.items[i_rank];

        // skip ahead using select blocks
        // for now, disable this functionality since we typically sample very finely, eg. every 1024 bits = every 32 blocks with 32-bit blocks.
        // const use_select_blocks_in_rank_queries = false;
        // if (use_select_blocks_in_rank_queries) {
        //     const select1_sr: usize = @as(BlockInt, 1) << self.config.select1_sr;
        //     while (self.decode_select_sample(count + select1_sr, self.s1.items, self.config.select1_sr)) |sample| {
        //         if (sample.block_index >= i_end) break;
        //         i_start = sample.block_index;
        //         count = sample.preceding_count;
        //     }
        // }

        // scan fully-covered blocks
        const blocks = bitmasks(self.bits);
        for (blocks[i_start..i_end]) |block| {
            count += @popCount(block);
        }

        // count relevant 1-bits in the last block
        const bit_offset = i & (BlockBits - 1);
        const block = blocks[i_end];
        const mask: ShiftInt = @intCast(bit_offset);
        count += @popCount(block & one_mask(mask));
        return count;
    }

    pub fn rank0(self: Self, i: usize) usize {
        assert(!self.has_multiplicity());
        if (i >= self.universe_size()) return self.num_zeros();
        return i - self.rank1(i);
    }

    pub fn select1(self: Self, n: usize) ?usize {
        if (n >= self.ones_count) return null;
        var sample = self.decode_select_sample(n, self.s1.items, self.config.select1_sr);

        // skip ahead using rank blocks
        var i_rank = sample.block_index << (self.config.rank_sr - ShiftBits) + 1;
        for (self.r.items[i_rank..], i_rank..) |count, i| {
            if (count > n) break;
            sample.block_index = i << (self.config.rank_sr - ShiftBits);
            sample.preceding_count = count;
        }

        // scan blocks until we find the one that contains the n-th 1-bit
        const blocks = bitmasks(self.bits);
        for (blocks[sample.block_index..]) |block| {
            const next_count = sample.preceding_count + @popCount(block);
            if (next_count > n) break;
            sample.preceding_count = next_count;
            sample.block_index += 1;
        }

        var block = blocks[sample.block_index];
        // Unset the k-1 preceding 1-bits
        for (sample.preceding_count..n) |_| block &= block - 1;
        return (sample.block_index << ShiftBits) + @ctz(block);
    }

    pub fn select0(self: Self, n: usize) ?usize {
        _ = self;
        _ = n;
        return null;
    }

    // return the information contained in the select sample that has the the `n`-th bit of its kind
    fn decode_select_sample(self: Self, n: usize, samples: []BlockInt, sr: ShiftInt) struct { block_index: usize, preceding_count: usize } {
        // if (n >= self.universe_size()) return null;
        assert(n < self.universe_size());
        const index = n >> sr;
        const sample = samples[index];
        const mask: BlockInt = BlockBits - 1;
        const cumulative_bits = sample & ~mask;
        const correction = sample & mask;
        const preceding_count = (index << sr) - correction;
        return .{ .block_index = cumulative_bits >> ShiftBits, .preceding_count = preceding_count };
    }

    pub fn num_ones(self: Self) usize {
        return self.ones_count;
    }

    pub fn num_zeros(self: Self) usize {
        return self.bits.capacity() - self.ones_count;
    }

    pub fn universe_size(self: Self) usize {
        return self.bits.capacity();
    }

    pub fn num_unique_ones(self: Self) usize {
        return self.num_ones();
    }

    pub fn num_unique_zeros(self: Self) usize {
        return self.num_zeros();
    }

    pub fn has_multiplicity(self: Self) bool {
        _ = self;
        return false;
    }

    pub fn deinit(self: *Self) void {
        self.r.deinit();
        self.s0.deinit();
        self.s1.deinit();
    }
};

fn bitmasks(bits: DynamicBitSet) []BlockInt {
    // ceil(bits.capacity() / BlockBits)
    const num_blocks = (bits.capacity() + (BlockBits - 1)) >> ShiftBits;
    return bits.unmanaged.masks[0..num_blocks];
}

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const alloc = gpa.allocator();

    const len = 123;

    var bits = try DynamicBitSet.initEmpty(alloc, len);
    defer bits.deinit();

    bits.set(100);

    const stdout = std.io.getStdOut().writer();

    try stdout.print("Hello, {}!\n", .{std.DynamicBitSetUnmanaged.ShiftInt});
    try stdout.print("Hello, {} {}!\n", .{ @typeInfo(usize).Int.bits, std.math.Log2Int(usize) });
    try stdout.print("Hello, {} {}!\n", .{ @typeInfo(u32).Int.bits, std.math.Log2Int(u32) });

    try stdout.print("Hello, {}!\n", .{std.DynamicBitSetUnmanaged.MaskInt});

    try stdout.print("Hello, {*}!\n", .{bits.unmanaged.masks});
}

fn one_mask(n: ShiftInt) BlockInt {
    assert(n <= @bitSizeOf(usize));
    return if (n == @bitSizeOf(usize))
        std.math.maxInt(usize)
    else
        (@as(BlockInt, 1) << n) - 1;
}

test "bitvec rank" {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const alloc = gpa.allocator();
    var bb = try DenseBitVecBuilder.init(alloc, 100);
    bb.one(50, 1);
    bb.one(99, 1);
    var bv = try bb.build(alloc, .{});
    try testing.expect(bv.num_ones() == 2);
    try testing.expect(bv.rank1(1000) == 2);
    try testing.expect(bv.rank1(50) == 0);
    try testing.expect(bv.rank1(51) == 1);
    try testing.expect(bv.rank1(98) == 1);
    try testing.expect(bv.rank1(99) == 1);
    try testing.expect(bv.rank1(100) == 2);
}

test "bitvec select" {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    const alloc = gpa.allocator();
    var bb = try DenseBitVecBuilder.init(alloc, 100);
    bb.one(50, 1);
    bb.one(99, 1);
    var bv = try bb.build(alloc, .{});
    try testing.expect(bv.select1(0).? == 50);
    try testing.expect(bv.select1(1).? == 99);
    try testing.expect(bv.select1(2) == null);
    try testing.expect(bv.select1(99) == null);
}
