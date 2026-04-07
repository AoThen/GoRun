cmake_minimum_required(VERSION 3.16)
project(MayeNano VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(cmake/FetchDependencies.cmake)

file(GLOB_RECURSE SOURCES "src/*.cpp")
file(GLOB_RECURSE HEADERS "src/*.h")

add_executable(MayeNano WIN32 ${SOURCES} ${HEADERS} res/resource.rc)

target_link_libraries(MayeNano
    PRIVATE imgui nlohmann_json d3d11 dxgi shell32 ole32 oleaut32 uuid)

target_include_directories(MayeNano PRIVATE "${CMAKE_SOURCE_DIR}/src")

# 应用程序图标（需要用户提供 icons/app.ico）
set(APP_ICON "${CMAKE_SOURCE_DIR}/res/icons/app.ico")
if(NOT EXISTS "${APP_ICON}")
    message(WARNING "Application icon not found: ${APP_ICON}")
    message(WARNING "Please provide an icon file for the application.")
endif()
