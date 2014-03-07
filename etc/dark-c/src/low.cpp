/*
*	Low-level coding routines
*	(C) kvark, Aug 2006
*/
#include "total.h"
#include "ptax.h"

/*	ark section	*/
#define BOT	(1<<14)
#define RESTUP	8

namespace ark	{
	tbord lo,hi,rng,code;
	FILE *f; char act;
	void nben()	{ fputc(lo>>24,f); }
	void nbde()	{ code = (code<<8)|fgetc(f); }
	void Set(char nact, FILE *nf)	{ act=nact; f=nf; }
	void Init()	{ lo=0; rng=(uint)(-1); }
	void StartDecode()	{ for(char i=0;i<4;i++) nbde(); }
	void FinalEncode()	{ for(char i=0;i<4;i++) nben(),lo<<=8; }
	void parse(tfreq,tfreq);
	void State(FILE *ff) { fprintf(ff,"\n\tLow:%8x Rng:%8x",lo,rng);}
}

void ark::parse(tfreq toff, tfreq tran)	{
	lo += rng*toff; rng *= tran;
	//main [en|de]coding loop
	do	{ hi = lo+rng;
		if((lo^hi)>=1<<24)	{
			if(rng>BOT) break;
			tbord lim = hi&0xFF000000;
			if(hi-lim >= lim-lo) lo=lim;
				else hi=lim-1;
		}do	{//shift
			act==MDEC ? nbde():nben();
			lo<<=8; hi<<=8;
		}while((lo^hi) < 1<<24);
		rng = hi-lo;
	}while(rng<BOT);
}
#undef BOT

/*	Eiler section	*/
int getlog(long ran)	{ int log;
	for(log=0; ran>>log; log++);
	return log;
}

#define FMB	12
#define FMAX	(1<<FMB)
#define getfreq(id) (2*t0[id]+t1[id])

void EilerCoder::Update(tfreq *tab, uchar log, uchar sd)	{
	tfreq add = (tab[0]>>sd)+5;
	tab[log] += add;
	if((tab[0] += add) >= FMAX)	{ int i;
		for(tab[0]=0,i=1; i<=LIN; i++)
			tab[0] += (++tab[i] >>= 1);
	}
}
void EilerCoder::Parse(tfreq off, uchar log)	{
	ark::parse(off, getfreq(log));
	Update(t0,log,a0);
	Update(t1,log,a1);
}
void EilerCoder::PutLog(int log)	{
	tfreq fcur; int i;
	ark::rng /= getfreq(0);
	for(fcur=0,i=1; i<log; i++)
		fcur += getfreq(i);
	Parse(fcur, log);
}
int EilerCoder::GetLog()	{ int log;
	ark::rng /= getfreq(0);
	tfreq fcur,val = (ark::code - ark::lo)/ark::rng;
	for(fcur=0,log=1; fcur+getfreq(log) <= val; log++)
		fcur += getfreq(log);
	Parse(fcur,log); return log;
}

void EilerCoder::Start(uchar na0, uchar na1, uchar nb0, uchar nb1)	{
	a0=na0; a1=na1; b0=nb0; b1=nb1;
	InitBits(cbit[0], NB);
}
void EilerCoder::InitBits(tfreq *v0, int num)	{
	num <<= 5; // for 32 bits
	while(num--) *v0++ = FMAX>>1;
}
void EilerCoder::InitFreq(locon *vr, int num)	{
	for(int i=0; i<num; i++)	{
		for(int j=1; j <= LIN; j++)
			vr[i][j] = 1;
		vr[i][0] = LIN;
	}
}
void EilerCoder::Finish()	{}

#undef getfreq
#define curfreq() ((u[0]+v[0])>>1)
#define LOBIT	(3+1)

//Encoding routine
void EilerCoder::EncodeEl(long num, int *pv)	{
	tfreq *u,*v; uchar log;
	for(log=0; num>>log; log++);
	if(log >= LIN)	{ tfreq fcur;
		uchar fl; PutLog(LIN); 
		for(u=r0,v=r1,fl=LIN; ;fl++,u++,v++)	{
			fcur = curfreq();
			ark::rng >>= FMB;
			if(fl == log) break;
			ark::parse(fcur, FMAX-fcur);
			u[0] -= u[0]>>b0;
			v[0] -= v[0]>>b1;
		}//stop bit
		ark::parse(0, fcur);
		u[0] += (FMAX-u[0])>>b0;
		v[0] += (FMAX-v[0])>>b1;
	}else PutLog(log);
	u = cbit[log];
	for(int i=log-2; i>=0; i--,u++)	{
		bool upd = (i>=log-LOBIT);
		ark::rng >>= FMB;
		if(num & (1<<i))	{
			ark::parse(u[0], FMAX-u[0]);
			if(upd) u[0] -= u[0]>>RESTUP;
		}else	{
			ark::parse(0, u[0]);
			if(upd) u[0] += (FMAX-u[0])>>RESTUP;
		}
	}pv[0]=log;
}
//Decoding routine
long EilerCoder::DecodeEl(int *pv)	{
	tbord val; ulong ran;
	tfreq *u,*v; uchar log;
	if((log = GetLog()) == LIN)	{ tfreq fcur;
		for(u=r0,v=r1; ;log++,u++,v++)	{
			ark::rng >>= FMB;
			fcur = curfreq();
			val = ark::code - ark::lo;
			if(val < fcur * ark::rng) break;
			ark::parse(fcur, FMAX-fcur);
			u[0] -= u[0]>>b0;
			v[0] -= v[0]>>b1;
		}//stop bit
		ark::parse(0, fcur);
		u[0] += (FMAX-u[0])>>b0;
		v[0] += (FMAX-v[0])>>b1;
	}//the rest
	u = cbit[log]; ran = 1<<(log-1);
	for(int i=log-2; i>=0; i--,u++)	{
		bool upd = (i>=log-LOBIT);
		ark::rng >>= FMB;
		val = ark::code - ark::lo;
		if(val >= u[0] * ark::rng)	{
			ran |= 1<<i;
			ark::parse(u[0], FMAX-u[0]);
			if(upd) u[0] -= u[0]>>RESTUP;
		}else	{
			ark::parse(0, u[0]);
			if(upd) u[0] += (FMAX-u[0])>>RESTUP;
		}
	}pv[0]=log;
	return ran;
}
#undef curfreq
#undef LOBIT
#undef FMB
#undef FMAX
#undef RESTUP
