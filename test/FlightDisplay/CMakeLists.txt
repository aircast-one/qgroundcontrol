target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        CameraControlTest.cc
        CameraControlTest.h
        FlightMapTest.cc
        FlightMapTest.h
        TelemetryChipsTest.cc
        TelemetryChipsTest.h
        TerrainProgressTest.cc
        TerrainProgressTest.h
        VideoTileTest.cc
        VideoTileTest.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
