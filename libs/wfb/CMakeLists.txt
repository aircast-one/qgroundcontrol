set(DEVOURER_JAGUAR1 ON CACHE BOOL "" FORCE)
set(DEVOURER_8814 OFF CACHE BOOL "" FORCE)
set(DEVOURER_JAGUAR2_8822B OFF CACHE BOOL "" FORCE)
set(DEVOURER_JAGUAR2_8821C OFF CACHE BOOL "" FORCE)
set(DEVOURER_JAGUAR3_8822C OFF CACHE BOOL "" FORCE)
set(DEVOURER_JAGUAR3_8822E ON CACHE BOOL "" FORCE)
set(DEVOURER_KESTREL_8852B OFF CACHE BOOL "" FORCE)
set(DEVOURER_KESTREL_8852C OFF CACHE BOOL "" FORCE)

add_subdirectory(devourer)

find_package(PkgConfig REQUIRED)
pkg_check_modules(WFB_LIBUSB REQUIRED IMPORTED_TARGET libusb-1.0)
pkg_check_modules(WFB_LIBSODIUM REQUIRED IMPORTED_TARGET libsodium)

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
        PkgConfig::WFB_LIBUSB
        PkgConfig::WFB_LIBSODIUM
        pcap
)
