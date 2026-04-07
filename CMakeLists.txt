cmake_minimum_required(VERSION 3.16)
project(GoRun VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include(cmake/FetchDependencies.cmake)

file(GLOB_RECURSE SOURCES "src/*.cpp")
file(GLOB_RECURSE HEADERS "src/*.h")

add_executable(GoRun WIN32 ${SOURCES} ${HEADERS} res/resource.rc)

target_link_libraries(GoRun
    PRIVATE imgui nlohmann_json d3d11 dxgi shell32 ole32 oleaut32 uuid gdiplus shlwapi)

target_include_directories(GoRun PRIVATE "${CMAKE_SOURCE_DIR}/src")

# MSVC 编译配置
if(MSVC)
    # 通用设置
    target_compile_options(GoRun PRIVATE
        /W3           # 警告级别 3
        /Zc:inline    # 移除未引用内联函数
    )
    
    # Release 模式优化配置
    set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} /O2 /Ob2 /Oi /Ot /GL /Gy")
    set(CMAKE_EXE_LINKER_FLAGS_RELEASE "${CMAKE_EXE_LINKER_FLAGS_RELEASE} /LTCG /OPT:REF /OPT:ICF")
    
    # Debug 模式配置（保持默认的 /RTC1 运行时检查）
    set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} /Zi")
    
    # 静态链接运行时库（避免 msvcrt 依赖）
    set_property(TARGET GoRun PROPERTY MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>")
endif()

# 应用程序图标（需要用户提供 icons/app.ico）
set(APP_ICON "${CMAKE_SOURCE_DIR}/res/icons/app.ico")
if(NOT EXISTS "${APP_ICON}")
    message(WARNING "Application icon not found: ${APP_ICON}")
    message(WARNING "Please provide an icon file for the application.")
endif()
