cmake_minimum_required(VERSION 3.25)
project(AircastCoreWfb LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../../libs/wfb wfb)
