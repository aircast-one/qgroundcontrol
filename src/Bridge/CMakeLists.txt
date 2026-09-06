target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        QGCBridgeCore.cc
        QGCBridgeCore.h
)

if(APPLE AND NOT IOS)
    target_sources(${CMAKE_PROJECT_NAME}
        PRIVATE
            QGCBridgeC.cc
            QGCBridgeC.h
            QGCEmbed.cc
            QGCLinksC.cc
            QGCLinksC.h
            QGCEntry.h
            QGCNativeDebugC.h
    )
endif()

target_include_directories(${CMAKE_PROJECT_NAME} PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
