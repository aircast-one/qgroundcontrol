set(DEVOURER_JAGUAR1 ON CACHE BOOL "" FORCE)
set(DEVOURER_8814 OFF CACHE BOOL "" FORCE)
set(DEVOURER_JAGUAR2_8822B OFF CACHE BOOL "" FORCE)
set(DEVOURER_JAGUAR2_8821C OFF CACHE BOOL "" FORCE)
set(DEVOURER_JAGUAR3_8822C OFF CACHE BOOL "" FORCE)
set(DEVOURER_JAGUAR3_8822E ON CACHE BOOL "" FORCE)
set(DEVOURER_KESTREL_8852B OFF CACHE BOOL "" FORCE)
set(DEVOURER_KESTREL_8852C OFF CACHE BOOL "" FORCE)

# A macOS release is a universal binary, but Homebrew ships libusb and libsodium
# as single-arch dylibs, so the x86_64 slice has nothing to link against and the
# app fails at the final link with undefined crypto_*/libusb_* symbols. Build
# both from source there instead: they then inherit CMAKE_OSX_ARCHITECTURES and
# come out universal. Everywhere else the system packages are fine.
list(LENGTH CMAKE_OSX_ARCHITECTURES WFB_OSX_ARCH_COUNT)
if(APPLE AND WFB_OSX_ARCH_COUNT GREATER 1)
    set(WFB_BUILD_DEPS ON)
else()
    set(WFB_BUILD_DEPS OFF)
endif()

if(WFB_BUILD_DEPS)
    CPMAddPackage(
        NAME libusb-cmake
        GITHUB_REPOSITORY libusb/libusb-cmake
        GIT_TAG v1.0.29
        OPTIONS
            "LIBUSB_BUILD_SHARED_LIBS OFF"
            "LIBUSB_BUILD_EXAMPLES OFF"
            "LIBUSB_BUILD_TESTING OFF"
            "LIBUSB_INSTALL_TARGETS OFF"
    )
    CPMAddPackage(
        NAME libsodium-cmake
        GITHUB_REPOSITORY robinlinden/libsodium-cmake
        GIT_TAG 9b2848dfc1b917a9410f0de9d81059b26cbfaa8d
        OPTIONS
            "SODIUM_DISABLE_TESTS ON"
    )
    add_library(WFB::libusb ALIAS usb-1.0)
    add_library(WFB::libsodium ALIAS sodium)
else()
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(WFB_LIBUSB REQUIRED IMPORTED_TARGET GLOBAL libusb-1.0)
    pkg_check_modules(WFB_LIBSODIUM REQUIRED IMPORTED_TARGET GLOBAL libsodium)
    add_library(WFB::libusb ALIAS PkgConfig::WFB_LIBUSB)
    add_library(WFB::libsodium ALIAS PkgConfig::WFB_LIBSODIUM)
endif()

set(DEVOURER_LIBUSB_TARGET WFB::libusb)

add_subdirectory(devourer)

file(GLOB WFBLINK_WIFI_SRC wifi/*.cpp)
file(GLOB WFBLINK_WFBNG_SRC wifi/wfb-ng/*.c wifi/wfb-ng/*.cpp)
if(NOT WIN32)
    file(GLOB WFBLINK_PLATFORM_SRC wifi/linux/*.cpp)
endif()

add_library(wfblink STATIC
    ${WFBLINK_WIFI_SRC}
    ${WFBLINK_WFBNG_SRC}
    ${WFBLINK_PLATFORM_SRC}
)

target_compile_features(wfblink PUBLIC cxx_std_20)

target_include_directories(wfblink
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}
        ${CMAKE_CURRENT_SOURCE_DIR}/wifi
        ${CMAKE_CURRENT_SOURCE_DIR}/devourer/src
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/wifi/compat
        ${CMAKE_CURRENT_SOURCE_DIR}/wifi/wfb-ng/include
        ${CMAKE_CURRENT_SOURCE_DIR}/devourer/hal
)

target_link_libraries(wfblink
    PUBLIC
        devourer
        WFB::libusb
        WFB::libsodium
        pcap
)
