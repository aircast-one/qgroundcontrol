list(LENGTH CMAKE_OSX_ARCHITECTURES QGC_RUST_ARCH_COUNT)
if(NOT MACOS OR QGC_RUST_ARCH_COUNT GREATER 1)
    message(STATUS "core-rs/: Rust core builds on single-arch macOS only for now; the bridge routes straight to Qt")
    return()
endif()

option(QGC_RUST_CORE "Route the bridge through the Rust core" ON)
find_program(QGC_CARGO cargo)
if(NOT QGC_RUST_CORE OR NOT QGC_CARGO)
    message(STATUS "core-rs/: Rust core off (QGC_RUST_CORE=${QGC_RUST_CORE}, cargo=${QGC_CARGO}); the bridge routes straight to Qt")
    return()
endif()

set(QGC_RUST_TARGET_DIR ${CMAKE_BINARY_DIR}/core-rs)
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(QGC_RUST_PROFILE debug)
    set(QGC_RUST_PROFILE_FLAG)
else()
    set(QGC_RUST_PROFILE release)
    set(QGC_RUST_PROFILE_FLAG --release)
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

add_library(qgc_core STATIC IMPORTED GLOBAL)
set_target_properties(qgc_core PROPERTIES IMPORTED_LOCATION ${QGC_RUST_LIB})
add_dependencies(${CMAKE_PROJECT_NAME} qgc_core_cargo)
target_link_libraries(${CMAKE_PROJECT_NAME} PRIVATE qgc_core)
target_compile_definitions(${CMAKE_PROJECT_NAME} PRIVATE QGC_RUST_CORE)
