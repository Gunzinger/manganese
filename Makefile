CFLAGS=-march=native -O3 -masm=intel -flto -fopenmp -std=gnu2x -ISIMDxorshift/include
CC?=cc

manganese: manganese.o tests-512.o tests-256.o tests.o hardware.o SIMDxorshift/simdxorshift128plus.o OpenBLAS/libopenblas.a
	$(CC) $(CFLAGS) -o manganese manganese.o tests-512.o tests-256.o tests.o hardware.o \
						SIMDxorshift/simdxorshift128plus.o OpenBLAS/libopenblas.a \

manganese.o: manganese.c
	$(CC) $(CFLAGS) -c manganese.c

tests-512.o: tests-512.c tests-512.h OpenBLAS/libopenblas.a
	$(CC) $(CFLAGS) -mrdrnd -mavx512f -mavx512bw -c tests-512.c

tests-256.o: tests-256.c tests-256.h OpenBLAS/libopenblas.a
	$(CC) $(CFLAGS) -mrdrnd -mavx2 -c tests-256.c

tests.o: tests.c tests.h
	$(CC) $(CFLAGS) -c tests.c

hardware.o: hardware.c hardware.h
	$(CC) $(CFLAGS) -c hardware.c

SIMDxorshift/simdxorshift128plus.o:
	$(MAKE) CFLAGS=-mavx512f -C SIMDxorshift simdxorshift128plus.o

OpenBLAS/libopenblas.a:
	$(MAKE) -C OpenBLAS USE_OPENMP=1 USE_THREAD=1 NO_LAPACK=1 DYNAMIC_ARCH=1 ONLY_CBLAS=1 NO_SHARED=1 USE_TLS=1
# DYNAMIC_LIST="HASWELL SKYLAKEX ATOM COOPERLAKE SAPPHIRERAPIDS ZEN" -> TODO: reinsert dynamic list with all avx2+ arches to slim down binary
# need to install a fortran compiler is ONLY_CBLAS=1 is removed 

.PHONY: clean
clean:
	rm -f manganese.iso *.o manganese
	rm -rf /tmp/manganese-fs /tmp/manganese-iso

.PHONY: clean-all
clean-all: clean
	rm -f *.tcz* tinycore.iso
	$(MAKE) -C SIMDxorshift clean
	$(MAKE) -C OpenBLAS clean

### Standalone ISO

manganese.iso: tinycore.iso manganese
	sudo bash build-iso.sh

tinycore.iso:
	curl -o tinycore.iso "http://www.tinycorelinux.net/14.x/x86_64/release/CorePure64-current.iso"
