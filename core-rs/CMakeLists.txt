option(QGC_RUST_CORE "Route the bridge through the Rust core" ON)
find_program(QGC_CARGO cargo)
list(LENGTH CMAKE_OSX_ARCHITECTURES QGC_RUST_ARCH_COUNT)

set(QGC_RUST_TARGET_DIR ${CMAKE_BINARY_DIR}/core-rs)
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(QGC_RUST_PROFILE debug)
    set(QGC_RUST_PROFILE_FLAG)
else()
    set(QGC_RUST_PROFILE release)
    set(QGC_RUST_PROFILE_FLAG --release)
endif()

if(ANDROID)
    find_program(QGC_CARGO_NDK cargo-ndk)
    if(NOT QGC_RUST_CORE OR NOT QGC_CARGO OR NOT QGC_CARGO_NDK)
        message(STATUS "core-rs/: Rust core off on Android (QGC_RUST_CORE=${QGC_RUST_CORE}, cargo=${QGC_CARGO}, cargo-ndk=${QGC_CARGO_NDK}); the bridge routes straight to Qt")
        return()
    endif()
    if(ANDROID_ABI STREQUAL "arm64-v8a")
        set(QGC_RUST_TRIPLE aarch64-linux-android)
    elseif(ANDROID_ABI STREQUAL "armeabi-v7a")
        set(QGC_RUST_TRIPLE armv7-linux-androideabi)
    elseif(ANDROID_ABI STREQUAL "x86_64")
        set(QGC_RUST_TRIPLE x86_64-linux-android)
    else()
        message(STATUS "core-rs/: no Rust target for ANDROID_ABI=${ANDROID_ABI}; the bridge routes straight to Qt")
        return()
    endif()
    if(NOT ANDROID_PLATFORM_LEVEL)
        string(REGEX REPLACE "^android-" "" ANDROID_PLATFORM_LEVEL "${ANDROID_PLATFORM}")
    endif()
    set(QGC_RUST_LIB ${QGC_RUST_TARGET_DIR}/${QGC_RUST_TRIPLE}/${QGC_RUST_PROFILE}/libqgc_core.a)
    add_custom_target(qgc_core_cargo
        COMMAND ${CMAKE_COMMAND} -E env ANDROID_NDK_HOME=${ANDROID_NDK_ROOT}
            ${QGC_CARGO} ndk --target ${ANDROID_ABI} --platform ${ANDROID_PLATFORM_LEVEL}
            build ${QGC_RUST_PROFILE_FLAG}
            --manifest-path ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml
            --target-dir ${QGC_RUST_TARGET_DIR}
        BYPRODUCTS ${QGC_RUST_LIB}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        COMMENT "cargo ndk build qgc-core (${ANDROID_ABI}, ${QGC_RUST_PROFILE})"
        VERBATIM
    )
else()
    if(NOT MACOS OR QGC_RUST_ARCH_COUNT GREATER 1 OR NOT QGC_RUST_CORE OR NOT QGC_CARGO)
        message(STATUS "core-rs/: Rust core off (QGC_RUST_CORE=${QGC_RUST_CORE}, cargo=${QGC_CARGO}, archs=${CMAKE_OSX_ARCHITECTURES}); the bridge routes straight to Qt")
        return()
    endif()
    set(QGC_RUST_LIB ${QGC_RUST_TARGET_DIR}/${QGC_RUST_PROFILE}/libqgc_core.a)
    add_custom_target(qgc_core_cargo
        COMMAND ${QGC_CARGO} build ${QGC_RUST_PROFILE_FLAG}
            --manifest-path ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml
            --target-dir ${QGC_RUST_TARGET_DIR}
        BYPRODUCTS ${QGC_RUST_LIB}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        COMMENT "cargo build qgc-core (${QGC_RUST_PROFILE})"
        VERBATIM
    )
endif()

add_library(qgc_core STATIC IMPORTED GLOBAL)
set_target_properties(qgc_core PROPERTIES IMPORTED_LOCATION ${QGC_RUST_LIB})
add_dependencies(${CMAKE_PROJECT_NAME} qgc_core_cargo)
target_link_libraries(${CMAKE_PROJECT_NAME} PRIVATE qgc_core)
target_compile_definitions(${CMAKE_PROJECT_NAME} PRIVATE QGC_RUST_CORE)
