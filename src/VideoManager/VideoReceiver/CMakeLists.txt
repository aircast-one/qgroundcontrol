# ============================================================================
# Video Receiver Subsystem
# Video streaming backends (GStreamer and Qt Multimedia)
# ============================================================================

target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        VideoReceiver.h
        VideoBackend.cc
        VideoBackend.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

add_subdirectory(GStreamer)
if(NOT QGC_HEADLESS_CORE)
    add_subdirectory(Offscreen)
    add_subdirectory(QtMultimedia)
endif()
# add_subdirectory(SceneGraph)
