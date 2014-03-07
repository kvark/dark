/*
*	Clip (c)kvark, Nov 2006
*	the command line processing class
*	Open simple-A version
*/
#include "total.h"
#include "clip.h"
#include <string.h>

#ifdef _WIN32
	const char slash = '\\';
	#define WIN32_LEAN_AND_MEAN
	#include <windows.h>
#else //UNIX
	const char slash = '/';
	#include <dirent.h>
	#include <sys/stat.h>
#endif


#define NAME	80
const char sign[] = "!dark";

const char err_output[]	= "\nCan't create output!";
const char err_input[]	= "\nCan't open input!";
const char err_sign[]	= "\nInvalid signature!";
	
//get file attribs
int attrib(const char *str)	{
	#ifdef _WIN32
	return GetFileAttributes(str);
	#else //UNIX
	struct stat sdata;
	if(stat(str, &sdata)) return -1;
	else return sdata.st_mode;
	#endif
}
//set full="./" or ".\\"
void finit(char *str)	{
	str[0]='.'; str[1]=slash; str[2]='\0';
}

/*	Clip class implementations		*/
/*	parses files and dirs, calls dark	*/

Clip::Clip()	{
	extern int block; //from Archon
	file = NULL; st.block = block = 0;
}

void Clip::ReadOpts(const char *pc)	{
	for(;pc[0]; pc++)	{ int bs;
		if(pc[0] == 'b')	{ bs = 0;
			while(pc++, *pc>='0' && *pc<='9')
				bs = 10*bs + pc[0] - '0';
			if(pc[0] == 'm') bs<<=20;
			else if(pc[0] == 'k') bs<<=10;
			if(bs>=4 && bs <= (1<<30))
				st.block = bs;
		}else break;
	}
}

int Clip::GetSign(FILE *ff)	{
	if(!ff) return 1;
	char buf[6] = {0};
	fread(buf,1,5,ff);
	if(feof(ff)) return 2;
	if(strcmp(buf,sign)) return 3;
	return 0;
}

int base;

bool Clip::EncodeFile(const char *path)	{
	//Prepare stage
	if(!st.block) st.block = 1<<22;	//default
	const char *pc = strrchr(path,slash);
	sprintf(full,"%s.dark",pc?pc+1:path);
	file = fopen(full,"r+b");
	if(!file)	{ //rewrite (!!)
		file = fopen(full,"w+b");
		if(!file) stopme(err_output,-2);
		fwrite(sign,1,5,file);
	}else return false;
	//EnCoding stage
	pc = strrchr(path,slash);
	if(pc)	{ base = ++pc-path;
		strncpy(full,path,base);
		full[base] = '\0';
	}else {base=0; finit(full);}
	pred.Prepare(MENC,file);
	fwrite(&st.block,sizeof(int),1,file);
	int attr = attrib(path);
	Reset(); if(attr == -1) return false;
	pred.fs = fopen(path,"rb");
	pred.Analyse();
	putstr(file, (char*)path+base);
	fwrite(&attr,sizeof(int),1,file);
	fwrite(&pred.len, sizeof(int),1,file);
	strcpy(full,path); outname();
	ark::Init(); pred.Compress();
	pred.Finish(); ark::FinalEncode();
	Finish(); return true;
}

bool Clip::DecodeFile(const char *path)	{
	//Prepare stage
	finit(full); base = 2;
	file = fopen(path,"rb");
	if(!file) stopme(err_input,-2);
	if(GetSign(file)) stopme(err_sign,-2);
	//DeCoding stage
	fread(&st.block, sizeof(int),1,file);
	pred.Prepare(MDEC,file);
	Reset(); int attr;//ready
	getstr(file,full+base);
	fread(&attr,sizeof(int),1,file);
	fread(&pred.len, sizeof(int),1,file);
	outname(); pred.fs = getoutfile(attr);
	ark::Init(); ark::StartDecode();
	pred.Extract(true); fclose(pred.fs);
	return true;
}

void Clip::Finish()	{
	if(!file) return;
	fclose(file); pred.Leave();
}

/*	private Clip routines	*/

FILE* Clip::getoutfile(int fat)	{
	FILE *ff = fopen(full+2,"wb");
	#ifdef _WIN32
	SetFileAttributes(full+2,fat);
	#else
	fchmod(ff->_fileno,fat);
	#endif
	return ff;
}
void Clip::outname()	{
	char *pf = full+base;
	if(strlen(pf)>=65) pf = strrchr(pf,slash);
	//char path[MAX_PATH]; CharToOem(pf,path);
	printf("\n\t%s  ",pf);
}
void Clip::putstr(FILE *ff, char *str)	{
	do putc(str[0],ff); while(*str++);
}
void Clip::getstr(FILE *ff, char *str)	{
	do str[0] = getc(ff); while(*str++);
}
