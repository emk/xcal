##
## Makefile for Xcaliber Mark II
##
## Copyright (C) 1997, Michael J. Fromberger, All Rights Reserved.
##
.PHONY: clean distclean dist

CC=gcc
CFLAGS=-Wall -Wno-unused-result -Werror -O2 -funsigned-char

SRCS=cmd.c con.c lang.c mem.c message.c net.c log.c port.c stream.c telnet.c\
	user.c val.c xhelp.c xcal.c
HDRS=$(filter-out xcal.h,$(SRCS:.c=.h)) default.h
OBJS=$(SRCS:.c=.o)
VERS=1.32
EXTRAS=README.md INSTALL INSTRUCT LICENSE Makefile help lang lib tests \
	xcal-history.html

TARGETS=xcal
LIBS=

.c.o: 
	$(CC) $(CFLAGS) -c $<

$(TARGETS):%: $(OBJS) %.o
	$(CC) $(CFLAGS) -o $@ $^ $(LIBS)

all: $(TARGETS)


TARBALL=README.md INSTALL INSTRUCT LICENSE Makefile ckcon $(HDRS) $(SRCS) \
	mem.h mem.c mkpriv startup xcal.c help/README help/*.hf \
	help/exptoc.x help/mktoc lang/*.lf lang/mklang tests

# Requires clang-format: https://clang.llvm.org/docs/ClangFormat.html
format:
	@ echo "Formatting C source files and headers ..."
	find . -type f -name '*.h' -o -name '*.c' -print0 | \
		xargs -0 clang-format --style=Google -i

clean:
	rm -f core *.o *~

distclean: clean
	rm -f $(TARGETS)

dist: distclean
	rm -f temp.tar
	tar -cvf temp.tar $(SRCS) $(HDRS) $(EXTRAS)
	mkdir xcal-$(VERS)
	cd xcal-$(VERS) && tar -xvf ../temp.tar
	find xcal-$(VERS) -type d -name CVS -prune -exec rm -rf {} \; -print
	tar -cvf xcal-$(VERS).tar xcal-$(VERS)
	gzip -9v xcal-$(VERS).tar
	rm -rf xcal-$(VERS)
	rm -f temp.tar

# Here there be dragons
