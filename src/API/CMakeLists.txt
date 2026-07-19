target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        DebugApiServer.cc
        DebugApiServer.h
        QGCCorePlugin.cc
        QGCCorePlugin.h
        QGCOptions.cc
        QGCOptions.h
        QmlComponentInfo.cc
        QmlComponentInfo.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

target_link_libraries(${CMAKE_PROJECT_NAME} PRIVATE Qt6::GuiPrivate)
