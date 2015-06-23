# makefile for dark

# sources
#OBJS				= obj/dark.o obj/clip.o obj/pred.o obj/archon3.o obj/low.o obj/ptax.o
TARGET				= bin/dark
MAKEFILE			= Makefile

# options
CC					= cc
#OUTPUT_OPTION		= -o $@
CFLAGS				= -O2 -finline-functions -funroll-loops -Wall -DNDEBUG
#CFLAGS				= -O0 -g -Wall
LDFLAGS				= -lstdc++
LDLIBS				= 
#TARGET_ARCH		=


# targets
.PHONY: all

all: $(TARGET)
$(TARGET): src/*.h src/*.cpp $(MAKEFILE)
	$(CC) -m32 -ggdb -o $(TARGET) $(LDFLAGS) src/*.cpp

verb: $(TARGET)
	$(CC) -m32 -ggdb -o bin/verb $(LDFLAGS) -DVERBOSE src/*.cpp

distclean: clean
clean:
	$(RM) $(TARGET)

pack: $(TARGET)
	./$(TARGET) p-r ../../data/book1
	ls -l book1.dark
	./$(TARGET) u-r book1.dark
	cmp book1 ../../data/book1
	rm book1.dark book1
