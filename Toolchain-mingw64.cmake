# Sample toolchain file for cross-compiling to Windows from Linux using MinGW-w64

set(CMAKE_SYSTEM_NAME Windows)
set(TOOLCHAIN_PREFIX x86_64-w64-mingw32)

# specify the cross compiler
set(CMAKE_C_COMPILER ${TOOLCHAIN_PREFIX}-gcc)
set(CMAKE_CXX_COMPILER ${TOOLCHAIN_PREFIX}-g++)
set(CMAKE_RC_COMPILER ${TOOLCHAIN_PREFIX}-windres)

# target environment on the build host system
set(CMAKE_FIND_ROOT_PATH /usr/${TOOLCHAIN_PREFIX})

# modify default behavior of FIND_XXX() commands
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)

# MinGW specifically needs these sometimes
set(CMAKE_SYSTEM_PROCESSOR x86_64)

# Standard static linking to avoid missing libgcc/libstdc++ dlls
set(CMAKE_CXX_STANDARD_LIBRARIES "-static-libgcc -static-libstdc++ -Wl,-Bstatic -lstdc++ -lpthread -Wl,-Bdynamic ${CMAKE_CXX_STANDARD_LIBRARIES}")
set(CMAKE_C_STANDARD_LIBRARIES "-static-libgcc -Wl,-Bstatic -lpthread -Wl,-Bdynamic ${CMAKE_C_STANDARD_LIBRARIES}")
