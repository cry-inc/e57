if (Test-Path build) { Remove-Item -Recurse -Force build }
if (Test-Path vcpkg) { Remove-Item -Recurse -Force vcpkg }
git clone https://github.com/Microsoft/vcpkg.git
./vcpkg/bootstrap-vcpkg.bat
mkdir build | out-null
cmake -B ./build -S . -DCMAKE_TOOLCHAIN_FILE="./vcpkg/scripts/buildsystems/vcpkg.cmake"
cmake --build ./build --config Release
./build/Release/main.exe
