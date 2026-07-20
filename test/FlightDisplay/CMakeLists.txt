target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        VideoTileTest.cc
        VideoTileTest.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
