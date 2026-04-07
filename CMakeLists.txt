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

# Release 模式优化配置
if(MSVC)
    # 代码优化
    target_compile_options(GoRun PRIVATE
        /O2      # 最大优化
        /Ob2     # 内联扩展
        /Oi      # 内联函数
        /Ot      # 速度优先
        /GL      # 全程序优化
        /Gy      # 函数级链接
        /Zc:inline  # 移除未引用内联函数
    )
    # 链接优化
    target_link_options(GoRun PRIVATE
        /LTCG    # 链接时代码生成
        /OPT:REF # 移除未引用函数
        /OPT:ICF # 合并相同 COMDAT
    )
    # 静态链接运行时库（避免 msvcrt 依赖）
    set_property(TARGET GoRun PROPERTY MSVC_RUNTIME_LIBRARY "MultiThreaded")
endif()

# 应用程序图标（需要用户提供 icons/app.ico）
set(APP_ICON "${CMAKE_SOURCE_DIR}/res/icons/app.ico")
if(NOT EXISTS "${APP_ICON}")
    message(WARNING "Application icon not found: ${APP_ICON}")
    message(WARNING "Please provide an icon file for the application.")
endif()
