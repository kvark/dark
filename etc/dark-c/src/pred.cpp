/*
*	Predator (c)kvark, Aug 2006
*	the single files processor
*	Open simple-A version
*/
#include "total.h"
#include "clip.h"
#include "ptax.h"
#include <malloc.h>

//number of zero terminating bytes
#define TERM	320
//must be approximately >= 2*DEEP+8

int BS; uchar *cbin;

/*
*	prepare - init Archon module,
*	count memory usage,
*	print setting information
*/
void Predator::Prepare(uchar act, FILE *ff)	{
	int memory = 0; BS = st.block;
	cbin = TERM + (s_bin = (uchar*)malloc(BS+TERM+1));
	memory += (BS+TERM+1)*sizeof(uchar);
	int rez = InitAll(BS,act,ff,&memory);
	if(!s_bin || rez)
		stopme("\nNot enough memory!",-2);
	printf("\nBlock: %dm", st.block>>20);
	printf(", Memory: %dMb", memory>>20);
	here = 0;
}
void Predator::Leave()	{ EndAll();
	if(s_bin) free(s_bin);
}

/*
*	extract - combine file from blocks
*/
void Predator::Extract(bool single)	{
	while(len)	{
		int n = DecodeBlock(cbin,fs); 
		len -= n; //rendicate(n); 
	}
}

/*
*	analyse - get file size:),
*	determine file type,
*	choose preprocessing modules.
*/
int Predator::Analyse()	{
	fseek(fs,0,SEEK_END);
	len = ftell(fs);
	fseek(fs,0,SEEK_SET);
	return 0;
}

/*
*	parseblock - encode block,
*	saving the border byte
*/
void ParseBlock(uchar *bin, int n)	{
	uchar save = bin[n];
	memset(bin-TERM,0,TERM);
	EncodeBlock(bin,n);
	bin[n] = save;
}

/*
*	compress - split file to blocks
*/
void Predator::Compress()	{
	if (st.entropy)	{
		Ptax px; px.Beready();
		int sym;
		while ((sym = fgetc(fs)) >= 0) {
			int dist = 0;
			fread(&dist,4,1,fs);
			Info("\nEncoding dist %d for sym %d", dist, sym);
			px.ran_encode(dist,sym);
		}
	}else	{
		while(len+here > BS)	{
			fread(cbin+here,1,BS-here,fs);
			len -= (BS-here); 
			ParseBlock(cbin,BS);
			here = 0;
		}//last piece
		fread(cbin+here,1,len,fs);
		here += len;
	}
	fclose(fs);
}

void Predator::Finish()	{
	if(!here) return;
	ParseBlock(cbin,here);
	here = 0;
}

#undef TERM
