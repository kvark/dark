/*!

bbb compression model

# Links

* http://mattmahoney.net/dc/bbb.cpp

*/

use std::io;
use compress::entropy::ari;
use super::{Symbol, SymContext};


/// State table:
///   nex(state, 0) = next state if bit y is 0, 0 <= state < 256
///   nex(state, 1) = next state if bit y is 1
///   nex(state, 2) = number of zeros in bit history represented by state
///   nex(state, 3) = number of ones represented
///
/// States represent a bit history within some context.
/// State 0 is the starting state (no bits seen).
/// States 1-30 represent all possible sequences of 1-4 bits.
/// States 31-252 represent a pair of counts, (n0,n1), the number
///   of 0 and 1 bits respectively.  If n0+n1 < 16 then there are
///   two states for each pair, depending on if a 0 or 1 was the last
///   bit seen.
/// If n0 and n1 are too large, then there is no state to represent this
/// pair, so another state with about the same ratio of n0/n1 is substituted.
/// Also, when a bit is observed and the count of the opposite bit is large,
/// then part of this count is discarded to favor newer data over old.
const STATE_TABLE: [[u8; 4]; 0x100] = [
    [  1,  2, 0, 0], [  3,  5, 1, 0], [  4,  6, 0, 1], [  7, 10, 2, 0], // 0-3
    [  8, 12, 1, 1], [  9, 13, 1, 1], [ 11, 14, 0, 2], [ 15, 19, 3, 0], // 4-7
    [ 16, 23, 2, 1], [ 17, 24, 2, 1], [ 18, 25, 2, 1], [ 20, 27, 1, 2], // 8-11
    [ 21, 28, 1, 2], [ 22, 29, 1, 2], [ 26, 30, 0, 3], [ 31, 33, 4, 0], // 12-15
    [ 32, 35, 3, 1], [ 32, 35, 3, 1], [ 32, 35, 3, 1], [ 32, 35, 3, 1], // 16-19
    [ 34, 37, 2, 2], [ 34, 37, 2, 2], [ 34, 37, 2, 2], [ 34, 37, 2, 2], // 20-23
    [ 34, 37, 2, 2], [ 34, 37, 2, 2], [ 36, 39, 1, 3], [ 36, 39, 1, 3], // 24-27
    [ 36, 39, 1, 3], [ 36, 39, 1, 3], [ 38, 40, 0, 4], [ 41, 43, 5, 0], // 28-31
    [ 42, 45, 4, 1], [ 42, 45, 4, 1], [ 44, 47, 3, 2], [ 44, 47, 3, 2], // 32-35
    [ 46, 49, 2, 3], [ 46, 49, 2, 3], [ 48, 51, 1, 4], [ 48, 51, 1, 4], // 36-39
    [ 50, 52, 0, 5], [ 53, 43, 6, 0], [ 54, 57, 5, 1], [ 54, 57, 5, 1], // 40-43
    [ 56, 59, 4, 2], [ 56, 59, 4, 2], [ 58, 61, 3, 3], [ 58, 61, 3, 3], // 44-47
    [ 60, 63, 2, 4], [ 60, 63, 2, 4], [ 62, 65, 1, 5], [ 62, 65, 1, 5], // 48-51
    [ 50, 66, 0, 6], [ 67, 55, 7, 0], [ 68, 57, 6, 1], [ 68, 57, 6, 1], // 52-55
    [ 70, 73, 5, 2], [ 70, 73, 5, 2], [ 72, 75, 4, 3], [ 72, 75, 4, 3], // 56-59
    [ 74, 77, 3, 4], [ 74, 77, 3, 4], [ 76, 79, 2, 5], [ 76, 79, 2, 5], // 60-63
    [ 62, 81, 1, 6], [ 62, 81, 1, 6], [ 64, 82, 0, 7], [ 83, 69, 8, 0], // 64-67
    [ 84, 71, 7, 1], [ 84, 71, 7, 1], [ 86, 73, 6, 2], [ 86, 73, 6, 2], // 68-71
    [ 44, 59, 5, 3], [ 44, 59, 5, 3], [ 58, 61, 4, 4], [ 58, 61, 4, 4], // 72-75
    [ 60, 49, 3, 5], [ 60, 49, 3, 5], [ 76, 89, 2, 6], [ 76, 89, 2, 6], // 76-79
    [ 78, 91, 1, 7], [ 78, 91, 1, 7], [ 80, 92, 0, 8], [ 93, 69, 9, 0], // 80-83
    [ 94, 87, 8, 1], [ 94, 87, 8, 1], [ 96, 45, 7, 2], [ 96, 45, 7, 2], // 84-87
    [ 48, 99, 2, 7], [ 48, 99, 2, 7], [ 88,101, 1, 8], [ 88,101, 1, 8], // 88-91
    [ 80,102, 0, 9], [103, 69,10, 0], [104, 87, 9, 1], [104, 87, 9, 1], // 92-95
    [106, 57, 8, 2], [106, 57, 8, 2], [ 62,109, 2, 8], [ 62,109, 2, 8], // 96-99
    [ 88,111, 1, 9], [ 88,111, 1, 9], [ 80,112, 0,10], [113, 85,11, 0], // 100-103
    [114, 87,10, 1], [114, 87,10, 1], [116, 57, 9, 2], [116, 57, 9, 2], // 104-107
    [ 62,119, 2, 9], [ 62,119, 2, 9], [ 88,121, 1,10], [ 88,121, 1,10], // 108-111
    [ 90,122, 0,11], [123, 85,12, 0], [124, 97,11, 1], [124, 97,11, 1], // 112-115
    [126, 57,10, 2], [126, 57,10, 2], [ 62,129, 2,10], [ 62,129, 2,10], // 116-119
    [ 98,131, 1,11], [ 98,131, 1,11], [ 90,132, 0,12], [133, 85,13, 0], // 120-123
    [134, 97,12, 1], [134, 97,12, 1], [136, 57,11, 2], [136, 57,11, 2], // 124-127
    [ 62,139, 2,11], [ 62,139, 2,11], [ 98,141, 1,12], [ 98,141, 1,12], // 128-131
    [ 90,142, 0,13], [143, 95,14, 0], [144, 97,13, 1], [144, 97,13, 1], // 132-135
    [ 68, 57,12, 2], [ 68, 57,12, 2], [ 62, 81, 2,12], [ 62, 81, 2,12], // 136-139
    [ 98,147, 1,13], [ 98,147, 1,13], [100,148, 0,14], [149, 95,15, 0], // 140-143
    [150,107,14, 1], [150,107,14, 1], [108,151, 1,14], [108,151, 1,14], // 144-147
    [100,152, 0,15], [153, 95,16, 0], [154,107,15, 1], [108,155, 1,15], // 148-151
    [100,156, 0,16], [157, 95,17, 0], [158,107,16, 1], [108,159, 1,16], // 152-155
    [100,160, 0,17], [161,105,18, 0], [162,107,17, 1], [108,163, 1,17], // 156-159
    [110,164, 0,18], [165,105,19, 0], [166,117,18, 1], [118,167, 1,18], // 160-163
    [110,168, 0,19], [169,105,20, 0], [170,117,19, 1], [118,171, 1,19], // 164-167
    [110,172, 0,20], [173,105,21, 0], [174,117,20, 1], [118,175, 1,20], // 168-171
    [110,176, 0,21], [177,105,22, 0], [178,117,21, 1], [118,179, 1,21], // 172-175
    [110,180, 0,22], [181,115,23, 0], [182,117,22, 1], [118,183, 1,22], // 176-179
    [120,184, 0,23], [185,115,24, 0], [186,127,23, 1], [128,187, 1,23], // 180-183
    [120,188, 0,24], [189,115,25, 0], [190,127,24, 1], [128,191, 1,24], // 184-187
    [120,192, 0,25], [193,115,26, 0], [194,127,25, 1], [128,195, 1,25], // 188-191
    [120,196, 0,26], [197,115,27, 0], [198,127,26, 1], [128,199, 1,26], // 192-195
    [120,200, 0,27], [201,115,28, 0], [202,127,27, 1], [128,203, 1,27], // 196-199
    [120,204, 0,28], [205,115,29, 0], [206,127,28, 1], [128,207, 1,28], // 200-203
    [120,208, 0,29], [209,125,30, 0], [210,127,29, 1], [128,211, 1,29], // 204-207
    [130,212, 0,30], [213,125,31, 0], [214,137,30, 1], [138,215, 1,30], // 208-211
    [130,216, 0,31], [217,125,32, 0], [218,137,31, 1], [138,219, 1,31], // 212-215
    [130,220, 0,32], [221,125,33, 0], [222,137,32, 1], [138,223, 1,32], // 216-219
    [130,224, 0,33], [225,125,34, 0], [226,137,33, 1], [138,227, 1,33], // 220-223
    [130,228, 0,34], [229,125,35, 0], [230,137,34, 1], [138,231, 1,34], // 224-227
    [130,232, 0,35], [233,125,36, 0], [234,137,35, 1], [138,235, 1,35], // 228-231
    [130,236, 0,36], [237,125,37, 0], [238,137,36, 1], [138,239, 1,36], // 232-235
    [130,240, 0,37], [241,125,38, 0], [242,137,37, 1], [138,243, 1,37], // 236-239
    [130,244, 0,38], [245,135,39, 0], [246,137,38, 1], [138,247, 1,38], // 240-243
    [140,248, 0,39], [249,135,40, 0], [250, 69,39, 1], [ 80,251, 1,39], // 244-247
    [140,252, 0,40], [249,135,41, 0], [250, 69,40, 1], [ 80,251, 1,40], // 248-251
    [140,252, 0,41], [  0,  0, 0, 0], [  0,  0, 0, 0], [  0,  0, 0, 0], // 253-255 are reserved
];

/// A StateMap maps a nonstationary counter state to a probability.
/// After each mapping, the mapping is adjusted to improve future
/// predictions.
struct StateMap {
    context: u8,
    table: [ari::apm::FlatProbability; 0x100],
}

impl StateMap {
    fn new() -> StateMap {
        let mut t = [0; 0x100];
        for i in 0 .. 0x100 {
            let mut n0 = STATE_TABLE[i][2] as usize;
            let mut n1 = STATE_TABLE[i][3] as usize;
            if n0 ==0 {
                n1 <<= 3;
            }
            if n1 == 0 {
                n0 <<= 3;
            }
            let pr = ((n1 + 1) << ari::apm::FLAT_BITS) / (n0 + n1 + 2);
            t[i] = pr as ari::apm::FlatProbability;
        }
        StateMap {
            context: 0,
            table: t,
        }
    }
    /// Trains by updating the previous prediction with y (0-1).
    pub fn update(&mut self, bit: u8, cx: u8) {
        let top = (bit as usize) << ari::apm::FLAT_BITS;
        let old = self.table[self.context as usize] as usize;
        let pr = (0xF * old + top + 0x8) >> 4;
        self.table[self.context as usize] = pr as ari::apm::FlatProbability;
        self.context = cx;
    }

    fn predict(&self) -> ari::apm::Bit {
        ari::apm::Bit::from_flat(self.table[self.context as usize])
    }
}


/// Coding model for BWT-DC output
pub struct Model {
    /// Context -> state
    ctx2state: [u8; 0x100],
    /// Context pointer
    ctx_id: u8,
    /// State -> probability
    state_map: StateMap,
    /// Bitwise context: last 0-7 bits with a leading 1 (1 - 0xFF)
    bit_context: u8,
    /// Last 4 whole bytes, last is in low 8 bits
    last_bytes: u32,
    /// Count of consecutive identical bytes (0 - 0xFFFF)
    run_count: u16,
    /// (0-3) if run is 0, 1, 2-3, 4+
    run_context: u32,
    gate1: Vec<(ari::apm::Gate, ari::apm::Gate)>,
    gate2: Vec<ari::apm::Gate>,
    gate3: Vec<ari::apm::Gate>,
    gate4: Vec<ari::apm::Gate>,
    gate5: Vec<ari::apm::Gate>,
}

struct UpdateCookie {
    b11: ari::apm::BinCoords,
    b12: ari::apm::BinCoords,
    b2: ari::apm::BinCoords,
    b3: ari::apm::BinCoords,
    b4: ari::apm::BinCoords,
    b5: ari::apm::BinCoords,
    c1: usize,
    c2: usize,
    c3: usize,
    c4: usize,
    c5: usize,
}

impl Model {
    /// Create a new model
    pub fn new() -> Model {
        Model {
            ctx2state: [0; 0x100],
            ctx_id: 0,
            state_map: StateMap::new(),
            bit_context: 1,
            last_bytes: 0,
            run_count: 0,
            run_context: 0,
            gate1: (0..0x100).map(|_| (ari::apm::Gate::new(), ari::apm::Gate::new())).collect(),
            gate2: (0..0x10000).map(|_| ari::apm::Gate::new()).collect(),
            gate3: (0..400).map(|_| ari::apm::Gate::new()).collect(),
            gate4: (0..0x2000).map(|_| ari::apm::Gate::new()).collect(),
            gate5: (0..0x4000).map(|_| ari::apm::Gate::new()).collect(),
        }
    }

    fn update(&mut self, bit: u8, reset: bool, cookie: UpdateCookie) {
        //model
        let state_old = self.ctx2state[self.ctx_id as usize];
        self.ctx2state[self.ctx_id as usize] = STATE_TABLE[state_old as usize][bit as usize];
        //context
        self.bit_context = ((self.bit_context & 0x7F) << 1) + bit;
        if reset {
            self.last_bytes = ((self.last_bytes & 0xFFFFFFFF) << 8) | (self.bit_context as u32);
            self.bit_context = 1;
            if (self.last_bytes >> 8) & 0xFF == self.bit_context as u32 {
                if self.run_count < 0xFFFF {
                    self.run_count += 1;
                }
                match self.run_count {
                    1 | 2 | 4 => self.run_context += 0x100,
                    _ => ()
                }
            } else {
                self.run_count = 0;
                self.run_context = 0;
            }
        }
        self.ctx_id = self.bit_context;
        //sub-update
        self.state_map.update(bit, self.ctx2state[self.ctx_id as usize]);
        let g1 = &mut self.gate1[cookie.c1];
        g1.0.update(bit!=0, cookie.b11, 1, 0);
        g1.1.update(bit!=0, cookie.b12, 5, 0);
        self.gate2[cookie.c2].update(bit!=0, cookie.b2, 3, 0);
        self.gate3[cookie.c3].update(bit!=0, cookie.b3, 4, 0);
        self.gate4[cookie.c4].update(bit!=0, cookie.b4, 3, 0);
        self.gate5[cookie.c5].update(bit!=0, cookie.b5, 3, 0);
    }

    fn predict(&self) -> (ari::apm::Bit, UpdateCookie) {
        let p0 = self.state_map.predict();
        let bit_context = self.bit_context as usize;
        let last_bytes = self.last_bytes as usize;
        let c1 = bit_context;
        let (p1, b11, b12) = {
            let g1 = &self.gate1[c1];
            let (p11, b11) = g1.0.pass(&p0);
            let (p12, b12) = g1.1.pass(&p0);
            let p1 = (p11.to_flat() + p12.to_flat() + 1) >> 1;
            (ari::apm::Bit::from_flat(p1), b11, b12)
        };
        let c2 = bit_context | ((last_bytes & 0xFF) << 8);
        let (p2, b2) = self.gate2[c2].pass(&p1);
        let c3 = (last_bytes & 0xFF) | (self.run_context as usize);
        let (p3, b3) = self.gate3[c3].pass(&p2);
        let c4 = bit_context | (last_bytes & 0x1F);
        let (p4x, b4) = self.gate4[c4].pass(&p3);
        let p4y = (p4x.to_flat() * 3 + p3.to_flat() + 2) >> 2;
        let p4 = ari::apm::Bit::from_flat(p4y);
        let c5y = bit_context ^ (last_bytes & 0xFFFFFF);
        let c5 = ((c5y * 123456791) & 0xFFFFFFFF) >> 18;
        let (p5x, b5) = self.gate5[c5].pass(&p4);
        let p5y = (p5x.to_flat() + p4.to_flat() + 1) >> 1;
        let p5 = ari::apm::Bit::from_flat(p5y);

        let pr = p5;
        let cookie = UpdateCookie {
            b11: b11,
            b12: b12,
            b2: b2,
            b3: b3,
            b4: b4,
            b5: b5,
            c1: c1,
            c2: c2,
            c3: c3,
            c4: c4,
            c5: c5,
        };

        (if pr.predict() {
            pr
        }else {
            // weird hack by MM, apparently coming from the fact
            // that the probability never reaches the upper bound
            let flat = pr.to_flat() + 1;
            ari::apm::Bit::from_flat(flat)
        }, cookie)
    }
}

impl super::Model<Symbol, SymContext> for Model {
    fn reset(&mut self) {
        *self = Model::new();
    }

    fn encode<W: io::Write>(&mut self, sym: Symbol, _ctx: &SymContext,
              eh: &mut ari::Encoder<W>) -> io::Result<()> {
        for i in (0..8).rev() {
            let bit = (sym >> i) & 1;
            let (prob, cookie) = self.predict();
            try!(eh.encode(bit != 0, &prob));
            self.update(bit, i == 0, cookie);
        }
        Ok(())
    }

    fn decode<R: io::Read>(&mut self, _ctx: &SymContext, dh: &mut ari::Decoder<R>)
              -> io::Result<Symbol> {
        let mut sym = 0 as Symbol;
        for i in (0..8).rev() {
            let (prob, cookie) = self.predict();
            let bit_b = try!(dh.decode(&prob));
            let bit = if bit_b {1} else {0};
            sym |= bit << i;
            self.update(bit, i == 0, cookie);
        }
        Ok(sym)
    }
}
