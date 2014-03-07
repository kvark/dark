/*
*	Dark-src-A (C)kvark, Nov 2006
*	the BWT-DC scheme universal compressor
*	*** Open Linux+Win32 version ***
*/

#include "total.h"
#include "clip.h"
#ifdef _WIN32
	#include <process.h>
#else //UNIX
	#include <unistd.h>
#endif

void stopme(const char msg[], int code)	{
	printf(msg);
	//printf("\n...[enter] to exit");
	//getchar();
	_exit(code);
}
char call_format[] =
	"dark <p[-switches]|u> <file>\n"
	"\tp - Pack, u - Unpack\n"
	"\t'bSize' set block size (like b5m or b2030k)\n"
	"Defaults: -b4mi1\nMemory usage: 5*block\n";

State st;

int main(int argc,char *argv[])	{
	Clip clip; long t0=clock();
	setbuf(stdout,NULL);
	printf("OpenDark ver A\t(C)kvark, 2006");
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
