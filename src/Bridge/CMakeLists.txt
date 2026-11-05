target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        QGCBridgeC.cc
        QGCBridgeC.h
        QGCHostNotices.cc
        QGCHostNotices.h
        QGCBridgeCore.cc
        QGCBridgeCore.h
        QGCCoreC.h
        QGCEntry.cc
        QGCEntry.h
        QGCQtThread.h
        QGCSpeechC.cc
        QGCSpeechC.h
        QGCVideoC.cc
        QGCVideoC.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
