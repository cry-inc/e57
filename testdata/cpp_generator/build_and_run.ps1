if (Test-Path build) { Remove-Item -Recurse -Force build }
if (Test-Path vcpkg) { Remove-Item -Recurse -Force vcpkg }
git clone https://github.com/Microsoft/vcpkg.git
cd vcpkg
git apply ../libe57format.diff
cd ..
./vcpkg/bootstrap-vcpkg.bat -disableMetrics
mkdir build | out-null
cmake -B ./build -S . -DCMAKE_TOOLCHAIN_FILE="./vcpkg/scripts/buildsystems/vcpkg.cmake"
cmake --build ./build --config Debug
./build/Debug/main.exe
