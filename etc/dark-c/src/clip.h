void stopme(const char[],int);

enum	{
	PRED_REVERSE	= 0x01,
	PRED_FRAGMENT	= 0x02,
	PRED_SOLID		= 0x04,
	PRED_ERROR		= 0x80,
};

/*
*	'Predator' class was originally created for
*	pre and post-processing techniques,
*	it deals with single files and blocks
*/

class Predator	{
	bool FindByte(FILE*,uchar);
	int Dezintegrate(int);
	uchar *s_bin;
public:
	int here,len;
	FILE *fs;
	void Prepare(uchar,FILE*);
	void Leave();
	void Extract(bool);
	int Analyse();
	void Compress();
	void Finish();
};

/*
*	'Clip' class was created to handle
*	input parameters, archive management
*	and filesystem surfing (folders)
*/

class Clip	{
	Predator pred;
	long time;
	FILE *file,*flis;
	char full[320];
	FILE *getoutfile(int);
	void outname();
	void putstr(FILE*,char*);
	void getstr(FILE*,char*);
	//void goback();
	int GetSign(FILE*);
	void Finish();
public: Clip();
	void ReadOpts(const char*);
	bool EncodeFile(const char*);
	bool DecodeFile(const char*);
};
