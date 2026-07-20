target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        DebugApiServer.cc
        DebugApiServer.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

# QPA input injection (qwindowsysteminterface.h) — the same mechanism QTest uses
target_link_libraries(${CMAKE_PROJECT_NAME} PRIVATE Qt6::GuiPrivate)
