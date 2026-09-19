find_program(QGC_CARGO cargo)
if(NOT QGC_CARGO)
    message(FATAL_ERROR "core-rs/: cargo was not found on PATH and the Rust core is part of this application. Install the Rust toolchain from https://rustup.rs and configure again.")
endif()

find_program(QGC_RUSTUP rustup)

function(qgc_rust_require_target triple)
    if(NOT QGC_RUSTUP)
        return()
    endif()
    execute_process(
        COMMAND ${QGC_RUSTUP} target list --installed
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        OUTPUT_VARIABLE installed
        OUTPUT_STRIP_TRAILING_WHITESPACE
        ERROR_QUIET
    )
    string(REPLACE "\n" ";" installed "${installed}")
    if(NOT "${triple}" IN_LIST installed)
        message(FATAL_ERROR "core-rs/: the Rust standard library for ${triple} is not installed. Run: rustup target add ${triple}")
    endif()
endfunction()

set(QGC_RUST_TARGET_DIR ${CMAKE_BINARY_DIR}/core-rs)
set(QGC_RUST_MANIFEST ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml)

file(GLOB_RECURSE QGC_RUST_SOURCES CONFIGURE_DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/src/*.rs)
list(APPEND QGC_RUST_SOURCES ${QGC_RUST_MANIFEST})
if(EXISTS ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock)
    list(APPEND QGC_RUST_SOURCES ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.lock)
endif()

if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(QGC_RUST_PROFILE debug)
    set(QGC_RUST_PROFILE_FLAG)
else()
    set(QGC_RUST_PROFILE release)
    set(QGC_RUST_PROFILE_FLAG --release)
endif()

if(ANDROID)
    find_program(QGC_CARGO_NDK cargo-ndk)
    if(NOT QGC_CARGO_NDK)
        message(FATAL_ERROR "core-rs/: cargo-ndk was not found on PATH and the Android build of the Rust core needs it. Run: cargo install cargo-ndk --locked")
    endif()
    if(ANDROID_ABI STREQUAL "arm64-v8a")
        set(QGC_RUST_TRIPLE aarch64-linux-android)
    elseif(ANDROID_ABI STREQUAL "armeabi-v7a")
        set(QGC_RUST_TRIPLE armv7-linux-androideabi)
    elseif(ANDROID_ABI STREQUAL "x86_64")
        set(QGC_RUST_TRIPLE x86_64-linux-android)
    elseif(ANDROID_ABI STREQUAL "x86")
        set(QGC_RUST_TRIPLE i686-linux-android)
    else()
        message(FATAL_ERROR "core-rs/: no Rust target is mapped to ANDROID_ABI=${ANDROID_ABI}. Add one to core-rs/CMakeLists.txt.")
    endif()
    qgc_rust_require_target(${QGC_RUST_TRIPLE})
    if(NOT ANDROID_PLATFORM_LEVEL)
        string(REGEX REPLACE "^android-" "" ANDROID_PLATFORM_LEVEL "${ANDROID_PLATFORM}")
    endif()
    if(NOT CMAKE_OBJCOPY)
        message(FATAL_ERROR "core-rs/: CMAKE_OBJCOPY is unset, so rust_eh_personality cannot be weakened and the Android link will collide with the GStreamer Rust plugins.")
    endif()
    set(QGC_RUST_LIB ${QGC_RUST_TARGET_DIR}/${QGC_RUST_TRIPLE}/${QGC_RUST_PROFILE}/libqgc_core.a)
    add_custom_command(
        OUTPUT ${QGC_RUST_LIB}
        COMMAND ${CMAKE_COMMAND} -E env ANDROID_NDK_HOME=${ANDROID_NDK_ROOT}
            ${QGC_CARGO} ndk --target ${ANDROID_ABI} --platform ${ANDROID_PLATFORM_LEVEL}
            build ${QGC_RUST_PROFILE_FLAG}
            --manifest-path ${QGC_RUST_MANIFEST}
            --target-dir ${QGC_RUST_TARGET_DIR}
        COMMAND ${CMAKE_OBJCOPY} --weaken-symbol=rust_eh_personality ${QGC_RUST_LIB}
        DEPENDS ${QGC_RUST_SOURCES}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        COMMENT "cargo ndk build qgc-core (${ANDROID_ABI}, ${QGC_RUST_PROFILE})"
        VERBATIM
    )
elseif(IOS)
    if(CMAKE_OSX_SYSROOT MATCHES "[Ss]imulator")
        if("${CMAKE_OSX_ARCHITECTURES}" MATCHES "x86_64")
            set(QGC_RUST_TRIPLE x86_64-apple-ios)
        else()
            set(QGC_RUST_TRIPLE aarch64-apple-ios-sim)
        endif()
    else()
        set(QGC_RUST_TRIPLE aarch64-apple-ios)
    endif()
    qgc_rust_require_target(${QGC_RUST_TRIPLE})
    set(QGC_RUST_LIB ${QGC_RUST_TARGET_DIR}/${QGC_RUST_TRIPLE}/${QGC_RUST_PROFILE}/libqgc_core.a)
    add_custom_command(
        OUTPUT ${QGC_RUST_LIB}
        COMMAND ${CMAKE_COMMAND} -E env IPHONEOS_DEPLOYMENT_TARGET=${CMAKE_OSX_DEPLOYMENT_TARGET}
            ${QGC_CARGO} build ${QGC_RUST_PROFILE_FLAG} --target ${QGC_RUST_TRIPLE}
            --manifest-path ${QGC_RUST_MANIFEST}
            --target-dir ${QGC_RUST_TARGET_DIR}
        DEPENDS ${QGC_RUST_SOURCES}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        COMMENT "cargo build qgc-core (${QGC_RUST_TRIPLE}, ${QGC_RUST_PROFILE})"
        VERBATIM
    )
else()
    list(LENGTH CMAKE_OSX_ARCHITECTURES QGC_RUST_ARCH_COUNT)
    if(APPLE AND QGC_RUST_ARCH_COUNT GREATER 1)
        set(QGC_RUST_SLICES)
        set(QGC_RUST_TARGET_FLAGS)
        foreach(arch IN LISTS CMAKE_OSX_ARCHITECTURES)
            if(arch MATCHES "^x86_64")
                set(triple x86_64-apple-darwin)
            elseif(arch STREQUAL "arm64")
                set(triple aarch64-apple-darwin)
            else()
                message(FATAL_ERROR "core-rs/: no Rust target is mapped to the macOS architecture ${arch}. Configure a single supported architecture or add a mapping to core-rs/CMakeLists.txt.")
            endif()
            qgc_rust_require_target(${triple})
            list(APPEND QGC_RUST_TARGET_FLAGS --target ${triple})
            list(APPEND QGC_RUST_SLICES ${QGC_RUST_TARGET_DIR}/${triple}/${QGC_RUST_PROFILE}/libqgc_core.a)
        endforeach()
        set(QGC_RUST_LIB ${QGC_RUST_TARGET_DIR}/universal/${QGC_RUST_PROFILE}/libqgc_core.a)
        add_custom_command(
            OUTPUT ${QGC_RUST_LIB}
            COMMAND ${QGC_CARGO} build ${QGC_RUST_PROFILE_FLAG} ${QGC_RUST_TARGET_FLAGS}
                --manifest-path ${QGC_RUST_MANIFEST}
                --target-dir ${QGC_RUST_TARGET_DIR}
            COMMAND ${CMAKE_COMMAND} -E make_directory ${QGC_RUST_TARGET_DIR}/universal/${QGC_RUST_PROFILE}
            COMMAND lipo -create ${QGC_RUST_SLICES} -output ${QGC_RUST_LIB}
            DEPENDS ${QGC_RUST_SOURCES}
            WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
            COMMENT "cargo build qgc-core (${CMAKE_OSX_ARCHITECTURES}, ${QGC_RUST_PROFILE})"
            VERBATIM
        )
    else()
        if(MSVC)
            set(QGC_RUST_LIB ${QGC_RUST_TARGET_DIR}/${QGC_RUST_PROFILE}/qgc_core.lib)
        else()
            set(QGC_RUST_LIB ${QGC_RUST_TARGET_DIR}/${QGC_RUST_PROFILE}/libqgc_core.a)
        endif()
        add_custom_command(
            OUTPUT ${QGC_RUST_LIB}
            COMMAND ${QGC_CARGO} build ${QGC_RUST_PROFILE_FLAG}
                --manifest-path ${QGC_RUST_MANIFEST}
                --target-dir ${QGC_RUST_TARGET_DIR}
            DEPENDS ${QGC_RUST_SOURCES}
            WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
            COMMENT "cargo build qgc-core (${QGC_RUST_PROFILE})"
            VERBATIM
        )
    endif()
endif()

add_custom_target(qgc_core_cargo DEPENDS ${QGC_RUST_LIB})

add_library(qgc_core STATIC IMPORTED GLOBAL)
set_target_properties(qgc_core PROPERTIES IMPORTED_LOCATION ${QGC_RUST_LIB})
if(UNIX AND NOT APPLE AND NOT ANDROID)
    set_property(TARGET qgc_core PROPERTY INTERFACE_LINK_LIBRARIES udev rt util pthread m dl)
elseif(WIN32)
    set_property(TARGET qgc_core PROPERTY INTERFACE_LINK_LIBRARIES
        advapi32 bcrypt cfgmgr32 dbghelp kernel32 ntdll setupapi userenv ws2_32)
endif()

add_dependencies(${CMAKE_PROJECT_NAME} qgc_core_cargo)
target_link_libraries(${CMAKE_PROJECT_NAME} PRIVATE qgc_core)
