#!/bin/bash
SDK_LIB="C:/Program Files (x86)/Windows Kits/10/Lib/10.0.26100.0"
SDK_INC="C:/Program Files (x86)/Windows Kits/10/Include/10.0.26100.0"
MSVC="C:/Program Files/Microsoft Visual Studio/2022/Community/VC/Tools/MSVC/14.44.35207"

export LIB="${MSVC}/lib/x64;${SDK_LIB}/um/x64;${SDK_LIB}/ucrt/x64"
export INCLUDE="${MSVC}/include;${SDK_INC}/ucrt;${SDK_INC}/um;${SDK_INC}/shared"
export PATH="${MSVC}/bin/Hostx64/x64:${PATH}"

cd "$(dirname "$0")"
cargo check "$@" 2>&1
