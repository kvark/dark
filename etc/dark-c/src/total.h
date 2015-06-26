#include <stdio.h>
#include <time.h>
#include <memory.h>
#include <assert.h>

typedef unsigned char uchar;
typedef unsigned short ushort;
typedef unsigned int uint;
typedef unsigned long ulong;

namespace ark	{
	void Set(char,FILE*);
	void Init();
	void StartDecode();
	void FinalEncode();
	void State(FILE*);
}
typedef int trax2[0x10001];
extern trax2 r;

void Info(const char *format, ...);
int InitAll(int,uchar,FILE*,int*);
void Reset();
void EndAll();
void EncodeBlock(uchar *,int);
uint DecodeBlock(uchar *,FILE*);

struct State	{
	int block;
	bool reverse;
	bool entropy;
};
extern struct State st;

#define MENC	0x01
#define MDEC	0x02
