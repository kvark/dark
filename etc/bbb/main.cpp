/* bbb.cpp - big block BWT compressor version 1, Aug. 31, 2006.
(C) 2006, Matt Mahoney, mmahoney (at) cs.fit.edu
This is free software under GPL, http://www.gnu.org/licenses/gpl.txt

To compress/decompress: bbb command input output

Commands are concatenated, e.g. 

  bbb cfqm10 in out = compress (c) in fast mode (f), quiet (q), 10 MiB block size.
  bbb df = decompress in fast mode.

Commands:
  c = compress (default).
  d = decompress.
  f = fast mode, needs 5x blocksize in memory, default is 1.25x blocksize.
  bN, kN, mN = blocksize N bytes, KiB, MiB (compression only), default = m4.
  q = quiet (no output except for errors).

The compression format is a memory efficient Burrows-Wheeler Transform (BWT) followed 
by a PAQ-like compression using a single order-0 context (no mixing) followed by 5 more
adaptive stages with low order contexts, and bitwise arithmetic coding.

LOW MEMORY BWT

The BWT is able to sort blocks as large as 80% of available memory in slow
mode or 20% in fast mode.  Using larger blocks generally improves compression,
especially for text.  In fast mode, the bytes of a block are sorted by their
right context (with wrap around) before compression with an order 0 model.  
For example, the block "banana" is sorted as follows:

           sorted  
     block context  T
     ----- ------- ---
  0    n    abana   a
  1    n    anaba   a
  2    b    anana   a  <- p (starting point for inverse transform)
  3    a    banan   b
  4    a    nanab   n
  5    a    naban   n

After decompression, the transform is inverted by making a sorted copy of the
block, T, then traversing the block as follows: from position p, find the
next byte in T, then find the matching byte in the block with the same rank,
i.e. if t[p] = the j'th occurrence of byte c, then set p = such that
block[p] = the j'th occurrence of c.  Repeat n times, where n = block size.
The initial value of p is the sorted position of the original first byte,
which must be sent by the compressor.

  Start at p = 2 (transmitted by the compressor).
  Output block[2] = 'b'
  T[2] contains the third 'a'.
  Find the third 'a' in block at position 5.
  Set p = 5.
  output block[5] = 'a'
  T[5] contains the second 'n'
  Find the second 'n' in block at position 1.
  Set p = 1.
  etc...

In fast mode, an array of n pointers into the block of n bytes is sorted
using std::stable_sort() (normally quicksort).  Each pointer uses 4 bytes
of memory, so the program needs 5n total memory.

In slow mode, the block is divided into 16 subblocks and the pointers are
sorted as usual.  Then the pointers are written to 16 temporary files and
merged.  This is fast because the pointers are accessed sequentially.
This requires n/4 bytes for the pointers, plus the original block (5n/4
total), and 4n bytes on disk.

To invert the transform in fast mode, a linked list is built, then traversed,
Note that T can be represented by just the cumulative counts of lexicographically
preceding values (a=0, b=3, c=4, d=4,..., n=4, o=6, ...).  Then the list is 
built by scanning the block and keeping a count of each value.

  block  ptr
  -----  ---
    n     0 -> 4 (count n=5)
    n     1 -> 5 (count n=6)
    b     2 -> 3 (count b=4)  <- p
    a     3 -> 0 (count a=1)
    a     4 -> 1 (count a=2)
    a     5 -> 2 (count a=3)

Then the list is traversed:

  2 -> 3 -> 0 -> 4 -> 1 -> 5
  b    a    n    a    n    a

The linked list requires 4 bytes for each pointer, so again it requires 5n
memory.  In slow mode, instead of building a list, the program searches the
block.  If block[p] is the j'th occurrence of c in T, then the program must
scan the block from the beginning and count j occurrences of c.  To make the
scan faster, the program builds an index of every 16'th occurrence of c, then
searches linearly from there.  The steps are:

  p = start (transmitted by compressor)
  Repeat n times
    Output block[p]
    Find c such that count[c] <= p < count[c+1] by binary search on count[]
    Let j = p - count[c]
    p = index[c][j/16]
    scan p forward (j mod 16) occurrences of c in block

A 2-D index would have variable row lengths, so it is organized into a 1 
dimensional array with each row c having length (count[c+1]-count[c])/16 + 1,
which is at most n/16 + 256 elements.  Each pointer is 4 bytes, so memory usage
is about 5n/4 including the block.  No temporary files are used.

ENTROPY CODING

BWT data is best coded with an order 0 model.  The transformed text tends to
have long runs of identical bytes (e.g. "nnbaaa").  The BWT data is modeled
with a modified PAQ with just one context (no mixing) followed by a 5 stage
SSE (APM) and bitwise arithmetic coding.  Modeling typically takes about
as much time as sorting and unsorting in slow mode.  The model uses about 5 MB
memory.

The order 0 model consists of a mapping: 

             order 1, 2, 3 contexts ----------+
                                              V
  order 0 context -> bit history -> p -> APM chain -> arithmetic coder
                  t1             sm

Bits are coded one at a time.  The arithmetic coder maintains a range
[lo, hi), initially [0, 1) and repeatedly subdivides the range in proportion
to p(0), p(1), the next bit probabilites predicted by the model.  The final
output is the shortest base 256 number x such that lo <= x < hi.  As the leading
bytes of x become known, they are output.  To decompress, the model predictions
are repeated as during compression, then the actual bit is determined by which
half of the subrange contains x.

The model inputs a bytewise order 0 context consisting of the last 0 to 7 bits
of the current byte, plus the number of bits.  There are a total of 255 possible
bitwise contexts.  For each context, a table (t1) maintains an 8 bit state
representing the history of 0 and 1 bits previously seen.  This history is mapped
by another table (a StateMap sm) to a probability, p, that the next bit will be 1.
This table is adaptive: after each prediction, the mapping (state -> p) is adjusted
to improve the last prediction.

The output of the StateMap is passed through a series of 6 more adaptive tables, 
(Adaptive Probability Maps, or APM) each of which maps a context and the input 
probability to an output probability.  The input probability is interpolated between
33 bins on a nonlinear scale with smaller bins near 0 and 1.  After each prediction,
the corresponding table entries on both sides of p are adjusted to improve the
last prediction.  The APM chain is like this:

      + A11 ->+            +--->---+ +--->---+
      |       |            |       | |       |
  p ->+       +-> A2 -> A3 +-> A4 -+-+-> A5 -+-> Encoder
      |       |
      + A12 ->+

A11 and A12 both take c0 (the preceding bits of the current byte) as additional 
context, but one is fast adapting and the other is slow adapting.  Their 
outputs are averaged.

A2 is an order 1 context (previous byte and current partial byte).

A3 takes the previous (but not current) byte as context, plus 2 bits that depend
on the current run length (0, 1, 2-3, or 4+), the number of times the last
byte was repeated.

A4 takes the current byte and the low 5 bits of the second byte back.
The output is averaged with 3/4 weight to the A3 output with 1/4 weight.

A5 takes a 14 bit hash of an order 3 context (last 3 bytes plus current partial
byte) and is averaged with 1/2 weight to the A4 output.

The StateMap, state table, APM, Encoder, and associated code (Array, squash(), 
stretch()) are taken from PAQ8 with minor non-functional changes (e.g. removing
global context).

*/

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <algorithm>
#define NDEBUG  // remove for debugging
#include <cassert>
using namespace std;

// 8, 16, 32 bit unsigned types
typedef unsigned char U8;
typedef unsigned short U16;
typedef unsigned int U32;

//////////////////////////// Array ////////////////////////////

// Array<T, ALIGN> a(n); creates n elements of T initialized to 0 bits.
// Constructors for T are not called.
// Indexing is bounds checked if assertions are on.
// a.size() returns n.
// a.resize(n) changes size to n, padding with 0 bits or truncating.
// Copy and assignment are not supported.
// Memory is aligned on a ALIGN byte boundary (power of 2), default is none.

template <class T, int ALIGN=0> class Array {
private:
  int n;     // user size
  int reserved;  // actual size
  char *ptr; // allocated memory, zeroed
  T* data;   // start of n elements of aligned data
  void create(int i);  // create with size i
public:
  explicit Array(int i=0) {create(i);}
  ~Array();
  T& operator[](int i) {
#ifndef NDEBUG
    if (i<0 || i>=n) fprintf(stderr, "%d out of bounds %d\n", i, n), exit(1);
#endif
    return data[i];
  }
  const T& operator[](int i) const {
#ifndef NDEBUG
    if (i<0 || i>=n) fprintf(stderr, "%d out of bounds %d\n", i, n), exit(1);
#endif
    return data[i];
  }
  int size() const {return n;}
  void resize(int i);  // change size to i
private:
  Array(const Array&);  // no copy or assignment
  Array& operator=(const Array&);
};

template<class T, int ALIGN> void Array<T, ALIGN>::resize(int i) {
  if (i<=reserved) {
    n=i;
    return;
  }
  char *saveptr=ptr;
  T *savedata=data;
  int saven=n;
  create(i);
  if (savedata && saveptr) {
    memcpy(data, savedata, sizeof(T)*min(i, saven));
    free(saveptr);
  }
}

template<class T, int ALIGN> void Array<T, ALIGN>::create(int i) {
  n=reserved=i;
  if (i<=0) {
    data=0;
    ptr=0;
    return;
  }
  const int sz=ALIGN+n*sizeof(T);
  ptr = (char*)calloc(sz, 1);
  if (!ptr) fprintf(stderr, "Out of memory\n"), exit(1);
  data = (ALIGN ? (T*)(ptr+ALIGN-(((long)ptr)&(ALIGN-1))) : (T*)ptr);
  assert((char*)data>=ptr && (char*)data<=ptr+ALIGN);
}

template<class T, int ALIGN> Array<T, ALIGN>::~Array() {
  free(ptr);
}

///////////////////////// state table ////////////////////////

// State table:
//   nex(state, 0) = next state if bit y is 0, 0 <= state < 256
//   nex(state, 1) = next state if bit y is 1
//   nex(state, 2) = number of zeros in bit history represented by state
//   nex(state, 3) = number of ones represented
//
// States represent a bit history within some context.
// State 0 is the starting state (no bits seen).
// States 1-30 represent all possible sequences of 1-4 bits.
// States 31-252 represent a pair of counts, (n0,n1), the number
//   of 0 and 1 bits respectively.  If n0+n1 < 16 then there are
//   two states for each pair, depending on if a 0 or 1 was the last
//   bit seen.
// If n0 and n1 are too large, then there is no state to represent this
// pair, so another state with about the same ratio of n0/n1 is substituted.
// Also, when a bit is observed and the count of the opposite bit is large,
// then part of this count is discarded to favor newer data over old.

static const U8 State_table[256][4]={
  {  1,  2, 0, 0},{  3,  5, 1, 0},{  4,  6, 0, 1},{  7, 10, 2, 0}, // 0-3
  {  8, 12, 1, 1},{  9, 13, 1, 1},{ 11, 14, 0, 2},{ 15, 19, 3, 0}, // 4-7
  { 16, 23, 2, 1},{ 17, 24, 2, 1},{ 18, 25, 2, 1},{ 20, 27, 1, 2}, // 8-11
  { 21, 28, 1, 2},{ 22, 29, 1, 2},{ 26, 30, 0, 3},{ 31, 33, 4, 0}, // 12-15
  { 32, 35, 3, 1},{ 32, 35, 3, 1},{ 32, 35, 3, 1},{ 32, 35, 3, 1}, // 16-19
  { 34, 37, 2, 2},{ 34, 37, 2, 2},{ 34, 37, 2, 2},{ 34, 37, 2, 2}, // 20-23
  { 34, 37, 2, 2},{ 34, 37, 2, 2},{ 36, 39, 1, 3},{ 36, 39, 1, 3}, // 24-27
  { 36, 39, 1, 3},{ 36, 39, 1, 3},{ 38, 40, 0, 4},{ 41, 43, 5, 0}, // 28-31
  { 42, 45, 4, 1},{ 42, 45, 4, 1},{ 44, 47, 3, 2},{ 44, 47, 3, 2}, // 32-35
  { 46, 49, 2, 3},{ 46, 49, 2, 3},{ 48, 51, 1, 4},{ 48, 51, 1, 4}, // 36-39
  { 50, 52, 0, 5},{ 53, 43, 6, 0},{ 54, 57, 5, 1},{ 54, 57, 5, 1}, // 40-43
  { 56, 59, 4, 2},{ 56, 59, 4, 2},{ 58, 61, 3, 3},{ 58, 61, 3, 3}, // 44-47
  { 60, 63, 2, 4},{ 60, 63, 2, 4},{ 62, 65, 1, 5},{ 62, 65, 1, 5}, // 48-51
  { 50, 66, 0, 6},{ 67, 55, 7, 0},{ 68, 57, 6, 1},{ 68, 57, 6, 1}, // 52-55
  { 70, 73, 5, 2},{ 70, 73, 5, 2},{ 72, 75, 4, 3},{ 72, 75, 4, 3}, // 56-59
  { 74, 77, 3, 4},{ 74, 77, 3, 4},{ 76, 79, 2, 5},{ 76, 79, 2, 5}, // 60-63
  { 62, 81, 1, 6},{ 62, 81, 1, 6},{ 64, 82, 0, 7},{ 83, 69, 8, 0}, // 64-67
  { 84, 71, 7, 1},{ 84, 71, 7, 1},{ 86, 73, 6, 2},{ 86, 73, 6, 2}, // 68-71
  { 44, 59, 5, 3},{ 44, 59, 5, 3},{ 58, 61, 4, 4},{ 58, 61, 4, 4}, // 72-75
  { 60, 49, 3, 5},{ 60, 49, 3, 5},{ 76, 89, 2, 6},{ 76, 89, 2, 6}, // 76-79
  { 78, 91, 1, 7},{ 78, 91, 1, 7},{ 80, 92, 0, 8},{ 93, 69, 9, 0}, // 80-83
  { 94, 87, 8, 1},{ 94, 87, 8, 1},{ 96, 45, 7, 2},{ 96, 45, 7, 2}, // 84-87
  { 48, 99, 2, 7},{ 48, 99, 2, 7},{ 88,101, 1, 8},{ 88,101, 1, 8}, // 88-91
  { 80,102, 0, 9},{103, 69,10, 0},{104, 87, 9, 1},{104, 87, 9, 1}, // 92-95
  {106, 57, 8, 2},{106, 57, 8, 2},{ 62,109, 2, 8},{ 62,109, 2, 8}, // 96-99
  { 88,111, 1, 9},{ 88,111, 1, 9},{ 80,112, 0,10},{113, 85,11, 0}, // 100-103
  {114, 87,10, 1},{114, 87,10, 1},{116, 57, 9, 2},{116, 57, 9, 2}, // 104-107
  { 62,119, 2, 9},{ 62,119, 2, 9},{ 88,121, 1,10},{ 88,121, 1,10}, // 108-111
  { 90,122, 0,11},{123, 85,12, 0},{124, 97,11, 1},{124, 97,11, 1}, // 112-115
  {126, 57,10, 2},{126, 57,10, 2},{ 62,129, 2,10},{ 62,129, 2,10}, // 116-119
  { 98,131, 1,11},{ 98,131, 1,11},{ 90,132, 0,12},{133, 85,13, 0}, // 120-123
  {134, 97,12, 1},{134, 97,12, 1},{136, 57,11, 2},{136, 57,11, 2}, // 124-127
  { 62,139, 2,11},{ 62,139, 2,11},{ 98,141, 1,12},{ 98,141, 1,12}, // 128-131
  { 90,142, 0,13},{143, 95,14, 0},{144, 97,13, 1},{144, 97,13, 1}, // 132-135
  { 68, 57,12, 2},{ 68, 57,12, 2},{ 62, 81, 2,12},{ 62, 81, 2,12}, // 136-139
  { 98,147, 1,13},{ 98,147, 1,13},{100,148, 0,14},{149, 95,15, 0}, // 140-143
  {150,107,14, 1},{150,107,14, 1},{108,151, 1,14},{108,151, 1,14}, // 144-147
  {100,152, 0,15},{153, 95,16, 0},{154,107,15, 1},{108,155, 1,15}, // 148-151
  {100,156, 0,16},{157, 95,17, 0},{158,107,16, 1},{108,159, 1,16}, // 152-155
  {100,160, 0,17},{161,105,18, 0},{162,107,17, 1},{108,163, 1,17}, // 156-159
  {110,164, 0,18},{165,105,19, 0},{166,117,18, 1},{118,167, 1,18}, // 160-163
  {110,168, 0,19},{169,105,20, 0},{170,117,19, 1},{118,171, 1,19}, // 164-167
  {110,172, 0,20},{173,105,21, 0},{174,117,20, 1},{118,175, 1,20}, // 168-171
  {110,176, 0,21},{177,105,22, 0},{178,117,21, 1},{118,179, 1,21}, // 172-175
  {110,180, 0,22},{181,115,23, 0},{182,117,22, 1},{118,183, 1,22}, // 176-179
  {120,184, 0,23},{185,115,24, 0},{186,127,23, 1},{128,187, 1,23}, // 180-183
  {120,188, 0,24},{189,115,25, 0},{190,127,24, 1},{128,191, 1,24}, // 184-187
  {120,192, 0,25},{193,115,26, 0},{194,127,25, 1},{128,195, 1,25}, // 188-191
  {120,196, 0,26},{197,115,27, 0},{198,127,26, 1},{128,199, 1,26}, // 192-195
  {120,200, 0,27},{201,115,28, 0},{202,127,27, 1},{128,203, 1,27}, // 196-199
  {120,204, 0,28},{205,115,29, 0},{206,127,28, 1},{128,207, 1,28}, // 200-203
  {120,208, 0,29},{209,125,30, 0},{210,127,29, 1},{128,211, 1,29}, // 204-207
  {130,212, 0,30},{213,125,31, 0},{214,137,30, 1},{138,215, 1,30}, // 208-211
  {130,216, 0,31},{217,125,32, 0},{218,137,31, 1},{138,219, 1,31}, // 212-215
  {130,220, 0,32},{221,125,33, 0},{222,137,32, 1},{138,223, 1,32}, // 216-219
  {130,224, 0,33},{225,125,34, 0},{226,137,33, 1},{138,227, 1,33}, // 220-223
  {130,228, 0,34},{229,125,35, 0},{230,137,34, 1},{138,231, 1,34}, // 224-227
  {130,232, 0,35},{233,125,36, 0},{234,137,35, 1},{138,235, 1,35}, // 228-231
  {130,236, 0,36},{237,125,37, 0},{238,137,36, 1},{138,239, 1,36}, // 232-235
  {130,240, 0,37},{241,125,38, 0},{242,137,37, 1},{138,243, 1,37}, // 236-239
  {130,244, 0,38},{245,135,39, 0},{246,137,38, 1},{138,247, 1,38}, // 240-243
  {140,248, 0,39},{249,135,40, 0},{250, 69,39, 1},{ 80,251, 1,39}, // 244-247
  {140,252, 0,40},{249,135,41, 0},{250, 69,40, 1},{ 80,251, 1,40}, // 248-251
  {140,252, 0,41}};  // 253-255 are reserved

#define nex(state,sel) State_table[state][sel]

///////////////////////////// Squash //////////////////////////////

// return p = 1/(1 + exp(-d)), d scaled by 8 bits, p scaled by 12 bits
int squash(int d) {
  static const int t[33]={
    1,2,3,6,10,16,27,45,73,120,194,310,488,747,1101,
    1546,2047,2549,2994,3348,3607,3785,3901,3975,4022,
    4050,4068,4079,4085,4089,4092,4093,4094};
  if (d>2047) return 4095;
  if (d<-2047) return 0;
  int w=d&127;
  d=(d>>7)+16;
  return (t[d]*(128-w)+t[(d+1)]*w+64) >> 7;
}

//////////////////////////// Stretch ///////////////////////////////

// Inverse of squash. d = ln(p/(1-p)), d scaled by 8 bits, p by 12 bits.
// d has range -2047 to 2047 representing -8 to 8.  p has range 0 to 4095.

class Stretch {
  Array<short> t;
public:
  Stretch();
  int operator()(int p) const {
    assert(p>=0 && p<4096);
    return t[p];
  }
} stretch;

Stretch::Stretch(): t(4096) {
  int pi=0;
  for (int x=-2047; x<=2047; ++x) {  // invert squash()
    int i=squash(x);
    for (int j=pi; j<=i; ++j)
      t[j]=x;
    pi=i+1;
  }
  t[4095]=2047;
}

//////////////////////////// StateMap //////////////////////////

// A StateMap maps a nonstationary counter state to a probability.
// After each mapping, the mapping is adjusted to improve future
// predictions.  Methods:
//
// sm.p(y, cx) converts state cx (0-255) to a probability (0-4095), 
//   and trains by updating the previous prediction with y (0-1).

// Counter state -> probability * 256
class StateMap {
protected:
  int cxt;  // context
  Array<U16> t; // 256 states -> probability * 64K
public:
  StateMap();
  int p(int y, int cx) {
    assert(cx>=0 && cx<t.size());
    t[cxt]+=(y<<16)-t[cxt]+128 >> 8;
    return t[cxt=cx] >> 4;
  }
};

StateMap::StateMap(): cxt(0), t(256) {
  for (int i=0; i<256; ++i) {
    int n0=nex(i,2);
    int n1=nex(i,3);
    if (n0==0) n1*=128;
    if (n1==0) n0*=128;
    t[i] = 65536*(n1+1)/(n0+n1+2);
  }
}

//////////////////////////// APM //////////////////////////////

// APM maps a probability and a context into a new probability
// that bit y will next be 1.  After each guess it updates
// its state to improve future guesses.  Methods:
//
// APM a(N) creates with N contexts, uses 66*N bytes memory.
// a.p(y, pr, cx, rate=8) returned adjusted probability in context cx (0 to
//   N-1).  rate determines the learning rate (smaller = faster, default 8).
//   Probabilities are scaled 12 bits (0-4095).  Update on last bit y (0-1).

class APM {
  int index;     // last p, context
  const int N;   // number of contexts
  Array<U16> t;  // [N][33]:  p, context -> p
public:
  APM(int n);
  int p(int y, int pr=2048, int cxt=0, int rate=8) {
    assert(pr>=0 && pr<4096 && cxt>=0 && cxt<N && rate>0 && rate<32);
    pr=stretch(pr);
    int g=(y<<16)+(y<<rate)-y-y;
    t[index] += g-t[index] >> rate;
    t[index+1] += g-t[index+1] >> rate;
    const int w=pr&127;  // interpolation weight (33 points)
    index=(pr+2048>>7)+cxt*33;
    return t[index]*(128-w)+t[index+1]*w >> 11;
  }
};

// maps p, cxt -> p initially
APM::APM(int n): index(0), N(n), t(n*33) {
  for (int i=0; i<N; ++i)
    for (int j=0; j<33; ++j)
      t[i*33+j] = i==0 ? squash((j-16)*128)*16 : t[j];
}


//////////////////////////// Predictor //////////////////////////

class Predictor {
  int pr;  // next return value of p() (0-4095)
public:
  Predictor(): pr(2048) {}
  int p() const {return pr;}
  void update(int y);
};

void Predictor::update(int y) {
  static int c0=1;  // bitwise context: last 0-7 bits with a leading 1 (1-255)
  static U32 c4=0;  // last 4 whole bytes, last is in low 8 bits
  static int bpos=0; // number of bits in c0 (0-7)
  static Array<U8> t1(256); // context -> state
  static StateMap sm;  // state -> pr
  static U8* cp=&t1[0];  // context pointer
  static int run=0;  // count of consecutive identical bytes (0-65535)
  static int runcxt=0;  // (0-3) if run is 0, 1, 2-3, 4+
  static APM a11(256), a12(256), a2(65536), a3(1024), a4(8192), a5(16384);

  // update model
  *cp=nex(*cp, y);

  // update context
  c0+=c0+y;
  if (++bpos==8) {
    bpos=0;
    c4=c4<<8|c0-256;
    c0=1;
    bpos=0;
    if (((c4^c4>>8)&255)==0) {
      if (run<65535) 
        ++run;
      if (run==1 || run==2 || run==4) runcxt+=256;
    }
    else run=0, runcxt=0;
  }

  // predict
  cp=&t1[c0];
  pr=sm.p(y, *cp);
  pr=a11.p(y, pr, c0, 5)+a12.p(y, pr, c0, 9)+1>>1;
  pr=a2.p(y, pr, c0|c4<<8&0xff00, 7);
  pr=a3.p(y, pr, c4&255|runcxt, 8);
  pr=a4.p(y, pr, c0|c4&0x1f00, 7)*3+pr+2>>2;
  pr=a5.p(y, pr, c0^(c4&0xffffff)*123456791>>18, 7)+pr+1>>1;
} 

//////////////////////////// Encoder ////////////////////////////

// An Encoder does arithmetic encoding.  Methods:
// Encoder(COMPRESS, f) creates encoder for compression to archive f, which
//   must be open past any header for writing in binary mode.
// Encoder(DECOMPRESS, f) creates encoder for decompression from archive f,
//   which must be open past any header for reading in binary mode.
// code(i) in COMPRESS mode compresses bit i (0 or 1) to file f.
// code() in DECOMPRESS mode returns the next decompressed bit from file f.
//   Global y is set to the last bit coded or decoded by code().
// compress(c) in COMPRESS mode compresses one byte.
// decompress() in DECOMPRESS mode decompresses and returns one byte.
// flush() should be called exactly once after compression is done and
//   before closing f.  It does nothing in DECOMPRESS mode.
// size() returns current length of archive
// setFile(f) sets alternate source to FILE* f for decompress() in COMPRESS
//   mode (for testing transforms).

typedef enum {COMPRESS, DECOMPRESS} Mode;
class Encoder {
private:
  Predictor predictor;
  const Mode mode;       // Compress or decompress?
  FILE* archive;         // Compressed data file
  U32 x1, x2;            // Range, initially [0, 1), scaled by 2^32
  U32 x;                 // Decompress mode: last 4 input bytes of archive
  FILE *alt;             // decompress() source in COMPRESS mode

  // Compress bit y or return decompressed bit
  int code(int y=0) {
    int p=predictor.p();
    assert(p>=0 && p<4096);
    p+=p<2048;
    U32 xmid=x1 + (x2-x1>>12)*p + ((x2-x1&0xfff)*p>>12);
    assert(xmid>=x1 && xmid<x2);
    if (mode==DECOMPRESS) y=x<=xmid;
    y ? (x2=xmid) : (x1=xmid+1);
    predictor.update(y);
    while (((x1^x2)&0xff000000)==0) {  // pass equal leading bytes of range
      if (mode==COMPRESS) putc(x2>>24, archive);
      x1<<=8;
      x2=(x2<<8)+255;
      if (mode==DECOMPRESS) x=(x<<8)+(getc(archive)&255);  // EOF is OK
    }
    return y;
  }

public:
  Encoder(Mode m, FILE* f);
  Mode getMode() const {return mode;}
  long size() const {return ftell(archive);}  // length of archive so far
  void flush();  // call this when compression is finished
  void setFile(FILE* f) {alt=f;}

  // Compress one byte
  void compress(int c) {
    assert(mode==COMPRESS);
    for (int i=7; i>=0; --i)
      code((c>>i)&1);
  }

  // Decompress and return one byte
  int decompress() {
    if (mode==COMPRESS) {
      assert(alt);
      return getc(alt);
    }
    else {
      int c=0;
      for (int i=0; i<8; ++i)
        c+=c+code();
      return c;
    }
  }
};

Encoder::Encoder(Mode m, FILE* f): 
    mode(m), archive(f), x1(0), x2(0xffffffff), x(0), alt(0) {
  if (mode==DECOMPRESS) {  // x = first 4 bytes of archive
    for (int i=0; i<4; ++i)
      x=(x<<8)+(getc(archive)&255);
  }
}

void Encoder::flush() {
  if (mode==COMPRESS)
    putc(x1>>24, archive);  // Flush first unequal byte of range
}


///////////////////////////////// BWT //////////////////////////////

// Globals
bool fast=false;  // transform method: fast uses 5x blocksize memory, slow uses 5x/4
int blockSize=0x400000;  // max BWT block size
int n=0;          // number of elements in block, 0 < n <= blockSize
Array<U8> block;  // [n] text to transform
Array<int> ptr;   // [n] or [n/16] indexes into block to sort
const int PAD=72; // extra bytes in block (copy of beginning)
int pos=0;        // bytes compressed/decompressed so far
bool quiet=false; // q option?

// true if block[a+1...] < block[b+1...] wrapping at n
inline bool lessthan(int a, int b) {
  if (a<0) return false;
  if (b<0) return true;
  int r=block[a+1]-block[b+1];  // an optimization
  if (r) return r<0;
  r=memcmp(&block[a+2], &block[b+2], PAD-8);
  if (r) return r<0;
  if (a<b) {
    int r=memcmp(&block[a+1], &block[b+1], n-b-1);
    if (r) return r<0;
    r=memcmp(&block[a+n-b], &block[0], b-a);
    if (r) return r<0;
    return memcmp(&block[0], &block[b-a], a)<0;
  }
  else {
    int r=memcmp(&block[a+1], &block[b+1], n-a-1);
    if (r) return r<0;
    r=memcmp(&block[0], &block[b+n-a], a-b);
    if (r) return r<0;
    return memcmp(&block[a-b], &block[0], b)<0;
  }

}

// read 4 byte value LSB first, or -1 at EOF
int read4(FILE* f) {
  unsigned int r=getc(f);
  r|=getc(f)<<8;
  r|=getc(f)<<16;
  r|=getc(f)<<24;
  return r;
}

// read n<=blockSize bytes from in to block, BWT, write to out
int encodeBlock(FILE* in, Encoder& en) {
  n=fread(&block[0], 1, blockSize, in);  // n = actual block size
  if (n<1) return 0;
  assert(block.size()>=n+PAD);
  for (int i=0; i<PAD; ++i) block[i+n]=block[i];

  // fast mode: sort the pointers to the block
  if (fast) {
    if (!quiet) printf("sorting     %10d to %10d  \r", pos, pos+n);
    assert(ptr.size()>=n);
    for (int i=0; i<n; ++i) ptr[i]=i;
    stable_sort(&ptr[0], &ptr[n], lessthan);  // faster than sort() or qsort()
    int p=min_element(&ptr[0], &ptr[n])-&ptr[0];
    en.compress(n>>24);
    en.compress(n>>16);
    en.compress(n>>8);
    en.compress(n);
    en.compress(p>>24);
    en.compress(p>>16);
    en.compress(p>>8);
    en.compress(p);
    if (!quiet) printf("compressing %10d to %10d  \r", pos, pos+n);
    for (int i=0; i<n; ++i) {
      en.compress(block[ptr[i]]);
      if (!quiet && i && (i&0xffff)==0) 
        printf("compressed  %10d of %10d  \r", pos+i, pos+n);
    }
    pos+=n;
    return n;
  }

  // slow mode: divide the block into 16 parts, sort them, write the pointers
  // to temporary files, then merge them.
  else {

    // write header
    if (!quiet) printf("writing header at %10d          \r", pos);
    int p=0;
    for (int i=1; i<n; ++i)
      if (lessthan(i, 0)) ++p;
    en.compress(n>>24);
    en.compress(n>>16);
    en.compress(n>>8);
    en.compress(n);
    en.compress(p>>24);
    en.compress(p>>16);
    en.compress(p>>8);
    en.compress(p);

    // sort pointers in 16 parts to temporary files
    const int subBlockSize = (n-1)/16+1;  // max size of sub-block
    int start=0, end=subBlockSize;  // range of current sub-block
    FILE* tmp[16];  // temporary files
    for (int i=0; i<16; ++i) {
      if (!quiet) printf("sorting      %10d to %10d  \r", pos+start, pos+end);
      tmp[i]=tmpfile();
      if (!tmp[i]) perror("tmpfile()"), exit(1);
      for (int j=start; j<end; ++j) ptr[j-start]=j;
      stable_sort(&ptr[0], &ptr[end-start], lessthan);
      for (int j=start; j<end; ++j) {  // write pointers
        int c=ptr[j-start];
        fprintf(tmp[i], "%c%c%c%c", c, c>>8, c>>16, c>>24);
      }
      start=end;
      end+=subBlockSize;
      if (end>n) end=n;
    }

    // merge sorted pointers
    if (!quiet) printf("merging      %10d to %10d  \r", pos, pos+n);
    unsigned int t[16];  // current pointers
    for (int i=0; i<16; ++i) {  // init t
      rewind(tmp[i]);
      t[i]=read4(tmp[i]);
    }
    for (int i=0; i<n; ++i) {  // merge and compress
      int j=min_element(t, t+16, lessthan)-t;
      en.compress(block[t[j]]);
      if (!quiet && i && (i&0xffff)==0) 
        printf("compressed  %10d of %10d  \r", pos+i, pos+n);
      t[j]=read4(tmp[j]);
    }
    for (int i=0; i<16; ++i)  // delete tmp files
      fclose(tmp[i]);
    pos+=n;
    return n;
  }
}

// forward BWT
void encode(FILE* in, Encoder& en) {
  block.resize(blockSize+PAD);
  if (fast) ptr.resize(blockSize+1);
  else ptr.resize((blockSize-1)/16+2);
  while (encodeBlock(in, en));
  en.compress(0);  // mark EOF
  en.compress(0);
  en.compress(0);
  en.compress(0);
}

// inverse BWT of one block
int decodeBlock(Encoder& en, FILE* out) {

  // read block size
  int n=en.decompress();
  n=n*256+en.decompress();
  n=n*256+en.decompress();
  n=n*256+en.decompress();
  if (n==0) return n;
  if (!blockSize) {  // first block?  allocate memory
    blockSize = n;
    if (!quiet) printf("block size = %d\n", blockSize);
    block.resize(blockSize+PAD);
    if (fast) ptr.resize(blockSize);
    else ptr.resize(blockSize/16+256);
  }
  else if (n<1 || n>blockSize) {
    printf("file corrupted: block=%d max=%d\n", n, blockSize);
    exit(1);
  }

  // read pointer to first byte
  int p=en.decompress();
  p=p*256+en.decompress();
  p=p*256+en.decompress();
  p=p*256+en.decompress();
  if (p<0 || p>=n) {
    printf("file corrupted: p=%d n=%d\n", p, n);
    exit(1);
  }

  // decompress and read block
  for (int i=0; i<n; ++i) {
    block[i]=en.decompress();
    if (!quiet && i && (i&0xffff)==0)
      printf("decompressed %10d of %10d  \r", pos+i, pos+n);
  }
  for (int i=0; i<PAD; ++i) block[i+n]=block[i];  // circular pad

  // count (sort) bytes
  if (!quiet) printf("unsorting    %10d to %10d  \r", pos, pos+n);
  Array<int> t(257);  // i -> number of bytes < i in block
  for (int i=0; i<n; ++i)
    ++t[block[i]+1];
  for (int i=1; i<257; ++i)
    t[i]+=t[i-1];
  assert(t[256]==n);

  // fast mode: build linked list
  if (fast) {
    for (int i=0; i<n; ++i)
      ptr[t[block[i]]++]=i;
    assert(t[255]==n);

    // traverse list
    for (int i=0; i<n; ++i) {
      assert(p>=0 && p<n);
      putc(block[p], out);
      p=ptr[p];
    }
    return n;
  }

  // slow: build ptr[t[c]+c+i] = position of i*16'th occurrence of c in block
  Array<int> count(256);  // i -> count of i in block
  for (int i=0; i<n; ++i) {
    int c=block[i];
    if ((count[c]++ & 15)==0)
      ptr[(t[c]>>4)+c+(count[c]>>4)]=i;
  }

  // decode
  int c=block[p];
  for (int i=0; i<n; ++i) {
    assert(p>=0 && p<n);
    putc(c, out);

    // find next c by binary search in t so that t[c] <= p < t[c+1]
    c=127;
    int d=64;
    while (d) {
      if (t[c]>p) c-=d;
      else if (t[c+1]<=p) c+=d;
      else break;
      d>>=1;
    }
    if (c==254 && t[255]<=p && p<t[256]) c=255;
    assert(c>=0 && c<256 && t[c]<=p && p<t[c+1]);

    // find approximate position of p
    int offset=p-t[c];
    const U8* q=&block[ptr[(t[c]>>4)+c+(offset>>4)]];  // start of linear search
    offset&=15;

    // find next p by linear search for offset'th occurrence of c in block
    while (offset--)
      if (*++q != c) q=(const U8*)memchr(q, c, &block[n]-q);
    assert(q && q>=&block[0] && q<&block[n]);
    p=q-&block[0];
  }
  pos+=n;
  return n;
}

// inverse BWT of file
void decode(Encoder& en, FILE* out) {
  while (decodeBlock(en, out));
}

/////////////////////////////// main ////////////////////////////

int main(int argc, char** argv) {
  clock_t start=clock();

  // check for args
  if (argc<4) {
    printf("bbb Big Block BWT file compressor, ver. 1\n"
      "(C) 2006, Matt Mahoney.  Free under GPL, http://www.gnu.org/licenses/gpl.txt\n"
      "\n"
      "To compress/decompress a file: bbb command input output\n"
      "\n"
      "Commands:\n"
      "c = compress (default),  d = decompress.\n"
      "f = fast mode, needs 5x block size memory, default uses 1.25x block size.\n"
      "q = quiet (no output except error messages).\n"
      "bN, kN, mN = use block size N bytes, KiB, MiB, default = m4 (compression only).\n"
      "\n"
      "Commands should be concatenated in any order, e.g. bbb cfm100q foo foo.bbb\n"
      "means compress foo to foo.bbb in fast mode using 100 MiB block size in quiet\n"
      "mode.\n");
    exit(0);
  }

  // read options
  Mode mode=COMPRESS;
  const char* p=argv[1];
  while (*p) {
    switch (*p) {
      case 'c': mode=COMPRESS; break;
      case 'd': mode=DECOMPRESS; break;
      case 'f': fast=true; break;
      case 'b': blockSize=atoi(p+1); break;
      case 'k': blockSize=atoi(p+1)<<10; break;
      case 'm': blockSize=atoi(p+1)<<20; break;
      case 'q': quiet=true; break;
    }
    ++p;
  }
  if (blockSize<1) printf("Block size must be at least 1\n"), exit(1);
  
  // open files
  FILE* in=fopen(argv[2], "rb");
  if (!in) perror(argv[2]), exit(1);
  FILE* out=fopen(argv[3], "wb");
  if (!out) perror(argv[3]), exit(1);

  // encode or decode
  if (mode==COMPRESS) {
    if (!quiet) printf("Compressing %s to %s in %s mode, block size = %d\n", 
      argv[2], argv[3], fast ? "fast" : "slow", blockSize);
    Encoder en(COMPRESS, out);
    encode(in, en);
    en.flush();
  }
  else if (mode==DECOMPRESS) {
    blockSize=0;
    if (!quiet) printf("Decompressing %s to %s in %s mode\n",
      argv[2], argv[3], fast ? "fast" : "slow");
    Encoder en(DECOMPRESS, in);
    decode(en, out);
  }
  if (!quiet) printf("%ld -> %ld in %1.2f sec                  \n", ftell(in), ftell(out),
    (clock()-start+0.0)/CLOCKS_PER_SEC);
  return 0;
}
