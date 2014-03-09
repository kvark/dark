typedef uint tbord;
typedef ushort tfreq;

int getlog(long);

#define NB	32
#define LIN	8
typedef tfreq bicon[32];
typedef tfreq locon[LIN+1];
extern int rb[256];

class EilerCoder	{
private:
	uchar a0,a1,b0,b1;
	bicon cbit[NB];
	void Update(tfreq*,uchar,uchar);
	void PutLog(int);
	int GetLog();
	void Parse(tfreq,uchar);
public:
	tfreq *t0,*t1;
	tfreq *r0,*r1;
	void Start(uchar,uchar,uchar,uchar);
	void InitBits(tfreq*,int);
	void InitFreq(locon*,int);
	void Finish();
	void EncodeEl(long,int*);
	long DecodeEl(int*);
};

class Ptax	{
	struct SYMBOL {
		long fir;
		uchar pref;
		//context
		long df;
		locon com;
		bicon bic;
	}sym[256];
	locon dis[4][NB];
	bicon sbc[NB];
	long las[256];
	long lp,tm;
	int num,arm;
	EilerCoder bc;
	//mtf operations
	uchar m[256],cat[256];
	int pov,was;
	void setcon(SYMBOL*);
	void postup(SYMBOL*,long);
public:
	uchar *bin;
	void ran_encode(ulong,uchar);
	ulong ran_decode(uchar);
	void Beready();
	void Perform(int*,uchar*,int);
	uint Decode(uchar*);
};
