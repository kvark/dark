/*
*	Ptax (C)kvark, Oct 2006
*	the DC transformer ver 4.0
*/
#include "total.h"
#include "ptax.h"

#define GRLOG	4
#define GRSIZE	(256>>GRLOG)
#define GRNUM	(1<<GRLOG)

int dc,ca[9] = {6,5,4,3,2,1,4,6,4};

void Ptax::setcon(SYMBOL *ps)	{
	dc = getlog(ps->df);
	if(dc>11) dc=11;
	int fl = (pov<2?0:(pov<8?1:2));
	bc.t0 = dis[fl][dc];
	bc.t1 = ps->com;
	bc.r0 = sbc[dc==11];
	bc.r1 = ps->bic;
	Info("\n\tSymbol context [%d][%d]", dc, fl);
}
void Ptax::postup(SYMBOL *ps, long num)	{
	int raz = pov-dc;
	int power = 0;
	if(raz < -6) power = 7;
	else if(raz >= 3) power = 3;
	else power = ca[6+raz];
	//ps->df = (ps->df + num)/2;
	ps->df += power*(num - ps->df)>>3;
	Info("\n\tUpdated avg dist to %d, using raz %d, dist %d and power %d", ps->df, raz, num, power);
	//fprintf(fd,"%lu\t%d\n",ran,ps-sym);
}

void Ptax::ran_encode(ulong ran, uchar cs)	{
	setcon(sym+cs);
	bc.EncodeEl(ran+1,&pov);
	postup(sym+cs,ran);
}
ulong Ptax::ran_decode(uchar cs)	{
	ulong ran; setcon(sym+cs);
	ran = bc.DecodeEl(&pov)-1;
	postup(sym+cs,ran);
	return ran;
}

void Ptax::Beready()	{ int i;
	for(i=0;i<256;i++)	{
		bc.InitFreq(&sym[i].com, 1);
		bc.InitBits(sym[i].bic, 1);
		sym[i].df = 1000;
	}pov = 2;
	for(i=0; i<4; i++)	{
		bc.InitFreq(dis[i], NB);
	}
	bc.InitBits(*sbc, NB);
	bc.Start(12,5,2,3);
}

void Ptax::Perform(int *r, uchar *bin, int n)	{
	for(int i=0; i<256; i++)
		sym[i].fir = las[i] = -1;
	num = 0; arm = 0; was = -1;
	int cp,cs=bin[0]; //main cycle
	memset(rb,0,sizeof(int)<<8);
	memset(r,-1,n*sizeof(int));
	bin[n] = bin[n-1]^1;
	for(cp=0; cp<n; )	{
		lp = las[cs]; rb[cs]++;
		if(lp == -1) { int i;
			sym[cs].fir = cp;
			for(i=-1; (i+=GRSIZE)<num; )
				cat[m[i]]++;
			for(i=++num; --i; ) m[i] = m[i-1];
		}else	{
			uchar cl=m[0],off,cur=cat[cs];
			for(off=0; cur--; off+=GRSIZE)	{
				uchar *mp = m+off+GRSIZE;
				uchar cla = m[off];
				cat[mp[-1]]++;
				m[off] = cl; cl = mp[0];
				register ulong rc;
				do	{ mp -= 4;
					rc = *(ulong*)mp;
					*(ulong*)(mp+1) = rc;
				}while(mp > m+off);
				m[off] = cla;
			}//the rest
			arm = off;
			if(cl != cs)	{
				register uchar ra=cl,rb;
				do	{ rb = m[++arm];
					m[arm] = ra; ra=rb;
				}while(ra != cs);
			}//attention!
			r[lp] = cp-lp-arm-1;
		}//remember
		cat[m[0] = was = cs] = 0;
		while((cs=bin[++cp]) == was);
		las[was] = cp-1;
	}//initial symbols
	for(cs=0; cs<256; cs++)	{
		ran_encode(rb[cs],0);
		if(rb[cs]) ran_encode(sym[cs].fir,cs);
	}
	if(num == 1) return;
	for(cp=0; cp<n; cp++)	{
		if(r[cp]>=0) ran_encode(r[cp],bin[cp]);
	}
}

uint Ptax::Decode(uchar *bot)	{
	int i,cs,n = ran_decode(0);
	if(!n) return 0;
	//read init & sort by dist
	for(num=0,cs=0; cs<256; cs++)	{
		rb[cs] = ran_decode(0);
		if(!rb[cs]) continue;
		las[cs] = lp = ran_decode(cs);
		for(i=num; i>0 && lp<las[m[i-1]]; i--)
			m[i] = m[i-1];
		m[i] = cs; num++;
	}
	if(num == 1)	{
		memset(bot,m[0],n); return n;
	}//read all others
	for(i=0; i<n;)	{
		int j,lim; ulong ra;
		cs = m[0]; tm = las[m[1]];
		while(i<tm) bot[i++] = cs;
		if(!--rb[cs]) tm = n;
		else tm += ran_decode(cs);
		//cmp border & move dword
		ra = *(ulong*)(m+1);
		for(j=0;;)	{
			if( (j+=4) >= num ) { lim=num; break; }
			if(tm+j <= las[m[j]]) { lim=j; break; }
			*(ulong*)(m+j-4) = ra;
			ra = *(ulong*)(m+j+1);
		}//the rest
		for(j-=3; j<lim && tm+j > las[m[j]]; j++)
			m[j-1] = m[j];
		las[ m[j-1]=cs ] = tm+j-1;
	}return n;
}

#undef GRLOG
#undef GRSIZE
#undef GRNUM
