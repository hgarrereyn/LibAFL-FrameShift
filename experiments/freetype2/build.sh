mkdir $OUT/seeds
# TRT/fonts is the full seed folder, but they're too big
cp TRT/fonts/TestKERNOne.otf $OUT/seeds/
cp TRT/fonts/TestGLYFOne.ttf $OUT/seeds/

tar xf libarchive-3.4.3.tar.xz

cd libarchive-3.4.3
./configure --disable-shared
make clean
make -j $(nproc)
make install
cd ..

cd freetype2
./autogen.sh
./configure --with-harfbuzz=no --with-bzip2=no --with-png=no --without-zlib
make clean
make all -j $(nproc)

$CXX $CXXFLAGS -std=c++11 -I include -I . src/tools/ftfuzzer/ftfuzzer.cc \
    objs/.libs/libfreetype.a $FUZZER_LIB -L /usr/local/lib -larchive \
    -o $OUT/ftfuzzer
