#pragma once

#include <string>

namespace mn {

enum class LogLevel {
    Debug,
    Info,
    Error
};

class Logger {
public:
    static void Init();
    static void Shutdown();
    static void Log(LogLevel level, const std::string& msg);
    static void LogW(LogLevel level, const std::wstring& msg);

private:
    static void Write(LogLevel level, const std::string& utf8Msg);
    static void EnsureOpen();
    static void* s_file;  // FILE* 通过 void* 避免暴露 stdio.h
};

} // namespace mn

// 便利宏
#define LOG_DEBUG(msg)    ::mn::Logger::Log(::mn::LogLevel::Debug, (msg))
#define LOG_INFO(msg)     ::mn::Logger::Log(::mn::LogLevel::Info, (msg))
#define LOG_ERROR(msg)    ::mn::Logger::Log(::mn::LogLevel::Error, (msg))
#define LOG_DEBUGW(msg)   ::mn::Logger::LogW(::mn::LogLevel::Debug, (msg))
#define LOG_INFOW(msg)    ::mn::Logger::LogW(::mn::LogLevel::Info, (msg))
#define LOG_ERRORW(msg)   ::mn::Logger::LogW(::mn::LogLevel::Error, (msg))