if(NOT MACOS)
    return()
endif()

enable_language(Swift)

set_directory_properties(PROPERTIES COMPILE_DEFINITIONS "")

add_library(QGCNativeUI STATIC
    Sources/AppShell.swift
    Sources/Bridge.swift
    Sources/Fact.swift
    Sources/SettingsPages.swift
    Sources/SettingsStore.swift
    Sources/SettingsWindow.swift
    Sources/ConnectionsSection.swift
    Sources/LinkConfigModel.swift
    Sources/Links.swift
    Sources/NativeDebug.swift
    Sources/NativeProbe.swift
    Sources/NativeWindow.swift
    Sources/QtHostWindow.swift
    Sources/Telemetry.swift
    Sources/TelemetryView.swift
)

set_target_properties(QGCNativeUI PROPERTIES
    AUTOMOC OFF
    AUTOUIC OFF
    AUTORCC OFF
)

# CMake does not pass the deployment target or architecture to swiftc, so the Swift
# objects claim the host OS and the linker warns that they outrun the app's 12.0
# floor. One -target per architecture is all swiftc accepts, so a universal build
# needs one Swift library per arch lipo'd together - not wired up yet.
list(LENGTH CMAKE_OSX_ARCHITECTURES QGC_SWIFT_ARCH_COUNT)
if(QGC_SWIFT_ARCH_COUNT GREATER 1)
    message(FATAL_ERROR "macos/: Swift cannot target ${CMAKE_OSX_ARCHITECTURES} in one library. Build per-arch and lipo, or drop to a single architecture.")
endif()

if(CMAKE_OSX_ARCHITECTURES)
    set(QGC_SWIFT_ARCH ${CMAKE_OSX_ARCHITECTURES})
else()
    execute_process(COMMAND uname -m OUTPUT_VARIABLE QGC_SWIFT_ARCH OUTPUT_STRIP_TRAILING_WHITESPACE)
endif()

target_compile_options(QGCNativeUI PRIVATE
    "$<$<COMPILE_LANGUAGE:Swift>:SHELL:-target ${QGC_SWIFT_ARCH}-apple-macosx${CMAKE_OSX_DEPLOYMENT_TARGET}>"
)

set(QGC_BRIDGE_DIR ${CMAKE_SOURCE_DIR}/src/Bridge)

target_compile_options(QGCNativeUI PRIVATE
    "$<$<COMPILE_LANGUAGE:Swift>:SHELL:-Xcc -fmodule-map-file=${QGC_BRIDGE_DIR}/module.modulemap>"
    "$<$<COMPILE_LANGUAGE:Swift>:SHELL:-Xcc -I${QGC_BRIDGE_DIR}>"
)

# Targeting the app's deployment floor makes swiftc emit back-deployment shims
# (libswiftCompatibility*). The Swift driver would find them; clang++ drives this
# link, so point it at the toolchain's static Swift libraries.
execute_process(
    COMMAND xcrun --find swiftc
    OUTPUT_VARIABLE QGC_SWIFTC_PATH
    OUTPUT_STRIP_TRAILING_WHITESPACE
)
get_filename_component(QGC_SWIFT_TOOLCHAIN "${QGC_SWIFTC_PATH}" DIRECTORY)
get_filename_component(QGC_SWIFT_TOOLCHAIN "${QGC_SWIFT_TOOLCHAIN}" DIRECTORY)

target_link_directories(${CMAKE_PROJECT_NAME} PRIVATE "${QGC_SWIFT_TOOLCHAIN}/lib/swift/macosx")

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

target_link_libraries(${CMAKE_PROJECT_NAME}
    PRIVATE
        QGCNativeUI
        "-framework AppKit"
        "-framework SwiftUI"
)
