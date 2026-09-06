if(NOT MACOS)
    return()
endif()

enable_language(Swift)

add_library(QGCNativeUI STATIC
    Sources/Bridge.swift
    Sources/NativeWindow.swift
    Sources/Telemetry.swift
    Sources/TelemetryView.swift
)

set_target_properties(QGCNativeUI PROPERTIES
    AUTOMOC OFF
    AUTOUIC OFF
    AUTORCC OFF
    COMPILE_DEFINITIONS ""
)

set(QGC_BRIDGE_DIR ${CMAKE_SOURCE_DIR}/src/Bridge)

target_compile_options(QGCNativeUI PRIVATE
    "$<$<COMPILE_LANGUAGE:Swift>:SHELL:-Xcc -fmodule-map-file=${QGC_BRIDGE_DIR}/module.modulemap>"
    "$<$<COMPILE_LANGUAGE:Swift>:SHELL:-Xcc -I${QGC_BRIDGE_DIR}>"
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

target_link_libraries(${CMAKE_PROJECT_NAME}
    PRIVATE
        QGCNativeUI
        "-framework AppKit"
        "-framework SwiftUI"
)
