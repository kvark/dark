# makefile for dark

# sources
#OBJS				= obj/dark.o obj/clip.o obj/pred.o obj/archon3.o obj/low.o obj/ptax.o
TARGET				= dark
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
	$(CC) -m32 -O0 -ggdb -o $(TARGET) $(LDFLAGS) src/*.cpp

distclean: clean
clean:
	$(RM) $(TARGET)
