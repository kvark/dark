/*
*	Archon3 - lite0 (C)kvark, Nov 2006
*	the Burrows-Wheeler Transformation algoritm
*/
#include "total.h"
#include "ptax.h"
#include <malloc.h>

//treshold for insertion sort
#define INSERT	10
//pointer conversion
#define p2b(pt) (*(ushort *)(pt))
#define p4b(pt) (*(ulong *)(pt))
//suffix in group to be direct-sorted
#define BADSUF(pid) (pid[0]>pid[1] && pid[0]>=pid[-1])

int isab(uchar*,uchar*);
void ray(int*,int*,uchar*);

int block,n;
int *p;
int baza,ch;
uchar *bin,*sfin;
int rb[0x100];
trax2 r,r2a;
Ptax px;

/*
*	step - update progress character
*/
int cur_step=0;
void step()	{
	char ar[] = {'-','\\','|','/'};
	putchar('\b'); putchar(ar[cur_step]);
	if(++cur_step == sizeof(ar)) cur_step=0;
}

void Reset()	{ px.Beready(); }

int InitAll(int NBS, uchar act, FILE *ff, int *mem)	{
	p = (int*)malloc(sizeof(int)*((block=NBS)+1));
	if(p == NULL) return -1;
	mem[0] += sizeof(r) + sizeof(rb) + sizeof(r2a);
	mem[0] += sizeof(px) + block*sizeof(int);
	#ifndef NDEBUG
	printf("\n*  ASSERTIONS ENABLED  *");
	#endif
	if(n == -1) return -1;
	ark::Set(act,ff); return 0;
}

void EndAll()	{ block = 0;
	if(p) free(p);
}

//reverse bytes order
void Reverse(register uchar *p0, register uchar *p1)	{
	while(p0+1 < p1)	{
		uchar tmp = *p0;
		*p0++ = *--p1; *p1 = tmp;
	}
}
//count cumulated frequences
void cumulate()	{ int i,cl;
	i=n; cl=256; do	{ cl--;
		i -= rb[cl], rb[cl] = i;
	}while(cl);
}

uint DecodeBlock(uchar *bin, FILE *fo)	{
	register int i,pos; step();
	if(!(n=px.Decode(bin))) return 0;
	baza = px.ran_decode(0);
	memset(rb, 0, 0x100*sizeof(int));
	step(); //Heh
	for(i=0; i<n; i++) rb[bin[i]]++;
	#define nextp(id) p[rb[bin[id]]++] = id
	cumulate(); nextp(baza);
	for(i=0; i<baza; i++)	nextp(i);
	for(i=baza+1; i<n; i++)	nextp(i);
	#undef nextp
	for(pos=baza,i=0; i<n; i++)	{
		putc(bin[pos = p[pos]],fo);
		assert(pos>=0 && pos<n);
	}return n;
}

void EncodeBlock(uchar *nbin, int nn)	{
	register uchar cl;
	register uchar *fly;
	register int i,pos,lim;
	bin = nbin; n = nn;
	px.ran_encode(n,0);
	if(!n) return; step();
	if(st.reverse) Reverse(bin,bin+n);
	baza=-1; // Radix sort
	memset(r, 0, sizeof(trax2));
	sfin = bin+n; //scans
	for(fly=bin; fly<sfin-1; fly++)
		r[p2b(fly)]++;
	r[ch=0x10000] = pos = n;
	i = 256; do	{ i--;
		cl=0; do	{
			pos -= r[--ch];
			r2a[ch] = r[ch] = pos;
		}while(--cl);
		rb[i] = pos;
		if((uchar)i == *bin)	{
			p[--pos] = 0; r[ch]--;
		}//for start
	}while(i);
	sfin[0] = 0xFF; fly=bin; //border
	do if(BADSUF(fly))
		p[r2a[p2b(fly)]++] = fly+1-bin, fly++;
	while(++fly<sfin);
	// Direct sort
	for(ch=0; ch<0x10000; ch++)	{
		ray(p+r[ch], p+r2a[ch], bin-5);
	}
	memcpy(r2a,r+1,sizeof(trax2)-sizeof(int));
	step(); *sfin=0xFF; //Right2Left wave
	cl=0; do	{ cl--;
		lim = r2a[(cl<<8)+cl];
		for(i=r[(uint)(cl+1)<<8]-1; i>=lim; i--)	{
			unsigned char cc = bin[pos = p[i]+1];
			if(cc <= cl) p[--r2a[(cc<<8)+cl]] = pos;
		}
		for(lim = r2a[(cl<<8)+cl]; i>=lim; i--)
			if(bin[pos = p[i]+1] == cl)
				p[--lim] = pos;
	}while(cl);
	sfin=(uchar*)p; //Left2Right wave
	cl=0; i=0; do	{
		ch = r[((int)cl+1)<<8]-r[cl<<8];
		assert(i == r[cl<<8]);
		for(; ch--; i++,sfin++)	{ uchar sym;
			if((pos = 1+p[i]) == n)	{
				//got '$' index
				baza = i; *sfin = *bin;
				continue;
			}//finish
			sym = *sfin = bin[pos];
			if(rb[sym] > i)	p[rb[sym]++] = pos;
		}
	}while(++cl);
	step(); memcpy(bin,p,n);
	px.Perform(p,bin,n);
	px.ran_encode(baza,0);
}


//compare 2 suffixes
int isab(register uchar *a,register uchar *b)	{
	int deep = 4+(a>b?b:a)-bin;
	while(deep>=0)	{
		if(p4b(a) != p4b(b)) break;
		a-=4; b-=4; deep-=4;
	}
	return p4b(a)>p4b(b) || (b<a && b<bin-4);
}
//choose medial of 3
int median(int a,int b,int c,uchar bof[])	{
	uint qa = p4b(a+bof), qb = p4b(b+bof), qc = p4b(c+bof);
	if(qa > qb)	return (qa<qc ? a : (qb>qc?b:c));
	else		return (qb<qc ? b : (qa>qc?a:c));
}
/* 
*	ray - the modified mkqsort
*	deep = bin-boff, dword comparsions
*/
void ray(int *A, int *B, register uchar *boff)	{
	register int *x,*y,*z;
	while(B-A > INSERT)	{
		int s = median(A[0],A[B-A>>1],B[-1],boff);
		ulong w = p4b(s+boff);
		x=y=A; z=B; do	{
			ulong q = p4b((s=*y)+boff);
			//check bound
			if(q < w || s+boff<=bin-4)	{
				*y++ = *x; *x++ = s;
			}else
			if(q > w)	{
				*y = *--z; *z = s; 
			}else y++;
		}while(y<z);
		if(A+1 < x) ray(A,x,boff);
		if(z+1 < B) ray(z,B,boff);
		A = x; B = z; boff-=4;
	}//insertion
	for(x=A+1; x<B; x++)	{
		int s = *(z=x);
		while(--z>=A && isab(boff+z[0],boff+s))
			z[1] = z[0];
		z[1] = s; //in place
	}
}
