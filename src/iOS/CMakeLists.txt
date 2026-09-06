if(NOT IOS)
    return()
endif()

target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        iOSSafeArea.h
        iOSSafeArea.mm
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(${CMAKE_PROJECT_NAME} PRIVATE "-framework UIKit")
