target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        AirUnitCameraControl.cc
        AirUnitCameraControl.h
        SubtitleWriter.cc
        SubtitleWriter.h
        VideoManager.cc
        VideoManager.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

add_subdirectory(VideoReceiver)
