target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/MultiSignalSpy.cc
        ${CMAKE_CURRENT_SOURCE_DIR}/MultiSignalSpy.h
        ${CMAKE_CURRENT_SOURCE_DIR}/PropertyTestHelper.h
        ${CMAKE_CURRENT_SOURCE_DIR}/UnitTest.cc
        ${CMAKE_CURRENT_SOURCE_DIR}/UnitTest.h
        ${CMAKE_CURRENT_SOURCE_DIR}/UnitTestTileGenerator.cc
        ${CMAKE_CURRENT_SOURCE_DIR}/UnitTestTileGenerator.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

add_subdirectory(BaseClasses)
add_subdirectory(Benchmarking)
add_subdirectory(Fixtures)
add_subdirectory(PropertyTesting)
add_subdirectory(QmlTesting)
add_subdirectory(Tests)

# The define differs from the one the project's precompiled header was built with, so GCC refuses
# the PCH for this file (-Winvalid-pch, fatal under -Werror) rather than compile it without.
set_source_files_properties(${CMAKE_CURRENT_SOURCE_DIR}/UnitTest.cc
    TARGET_DIRECTORY ${CMAKE_PROJECT_NAME}
    PROPERTIES COMPILE_DEFINITIONS QT_TESTCASE_SOURCEDIR="${CMAKE_SOURCE_DIR}" SKIP_PRECOMPILE_HEADERS ON
)
