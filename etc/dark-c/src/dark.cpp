/*
*	Dark-0.51 (C)kvark, Nov 2006
*	the BWT-DC scheme universal compressor
*/

#include "total.h"
#include "clip.h"
#ifdef _WIN32
	#include <process.h>
#else //UNIX
	#include <unistd.h>
#endif
#include <cstdarg>

#ifdef VERBOSE
void Info(const char *format, ...)	{
	va_list args;
	va_start(args, format);
	vprintf(format, args);
	va_end(args);
}
#else
void Info(const char *format, ...)	{}
#endif


void stopme(const char msg[], int code)	{
	printf(msg);
	//printf("\n...[enter] to exit");
	//getchar();
	_exit(code);
}
char call_format[] =
	"\ndark <p[-switches]|l|u> <file>"
	"\n\tp - Pack, l - List, u - Unpack"
	"\n\t'bSize' set block size (like b5m or b2030k)"
	"\n\t'r' don't reverse sorting order"
	"\n\t'e' consume distance list directly (test the entropy model)"
	"\n\t'i<0|1|2>' silent/normal/extra info"
	"\nDefaults: -b4mi1\nMemory usage: 5*block";

State st;

int main(int argc,char *argv[])	{
	Clip clip; long t0=clock();
	setbuf(stdout,NULL);
	printf("Dark v0.51\t(C)kvark, 2006");
	printf("\nAdvanced command line BWT-DC compressor");
	if(argc < 3) stopme(call_format,-1);
	if(argv[1][0] == 'p')	{
		if(argv[1][1] == '-')
			clip.ReadOpts(argv[1]+2);
		clip.EncodeFile(argv[2]);
	}else if(argv[1][0] == 'u')	{
		clip.DecodeFile(argv[2]);
	}else stopme("\nUnknow command!",0);
	t0 = 1000*(clock()-t0)/CLOCKS_PER_SEC;
	printf("\nTime wasted: %d.%3ds\n",t0/1000,t0%1000);
	return 0;
}
