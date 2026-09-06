target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        DebugApiServer.cc
        DebugApiServer.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

# QPA input injection (qwindowsysteminterface.h) — the same mechanism QTest uses
target_link_libraries(${CMAKE_PROJECT_NAME} PRIVATE Qt6::GuiPrivate Qt6::Test)

# The debug API can read any QObject property and drive the vehicle, so a shipped build should
# have to opt in rather than opt out. Defaults ON to keep every existing workflow working; a
# release pipeline sets -DQGC_ENABLE_DEBUG_API=OFF and startIfConfigured() compiles away, so the
# environment variable cannot switch it back on.
option(QGC_ENABLE_DEBUG_API "Allow QGC_DEBUG_API_PORT to start the localhost debug API" ON)
if(QGC_ENABLE_DEBUG_API)
    target_compile_definitions(${CMAKE_PROJECT_NAME} PRIVATE QGC_ENABLE_DEBUG_API)
endif()
