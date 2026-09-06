target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        DebugApiServerTest.cc
        DebugApiServerTest.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
# loadTestView() lives with the QML interaction helpers
target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/../QmlControls)
