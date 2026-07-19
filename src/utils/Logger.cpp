#include "Logger.h"
#include "utils/PathUtils.h"
#include "utils/StringUtils.h"
#include <cstdio>
#include <ctime>
#include <string>

namespace mn {

void* Logger::s_file = nullptr;

static const char* LevelToString(LogLevel level) {
    switch (level) {
        case LogLevel::Debug: return "DEBUG";
        case LogLevel::Info:  return "INFO";
        case LogLevel::Error: return "ERROR";
        default:              return "UNKNOWN";
    }
}

static std::string GetTimestamp() {
    std::time_t now = std::time(nullptr);
    std::tm tm;
    localtime_s(&tm, &now);
    char buf[32];
    std::strftime(buf, sizeof(buf), "%Y-%m-%d %H:%M:%S", &tm);
    return buf;
}

void Logger::EnsureOpen() {
    if (s_file) return;
    std::wstring logPath = PathUtils::GetExeDir() + L"\\GoRun.log";
    s_file = _wfopen(logPath.c_str(), L"ab");
    if (!s_file) {
        return;
    }
    // 仅在新文件时写入 BOM 头
    fseek(static_cast<FILE*>(s_file), 0, SEEK_END);
    if (ftell(static_cast<FILE*>(s_file)) == 0) {
        fputs("\xEF\xBB\xBF", static_cast<FILE*>(s_file));
    }
}

void Logger::Init() {
    EnsureOpen();
}

void Logger::Shutdown() {
    if (s_file) {
        fclose(static_cast<FILE*>(s_file));
        s_file = nullptr;
    }
}

void Logger::Write(LogLevel level, const std::string& utf8Msg) {
    EnsureOpen();
    if (!s_file) return;
    std::string line = "[" + GetTimestamp() + "] [" + LevelToString(level) + "] " + utf8Msg + "\n";
    fputs(line.c_str(), static_cast<FILE*>(s_file));
    fflush(static_cast<FILE*>(s_file));
}

void Logger::Log(LogLevel level, const std::string& msg) {
    Write(level, msg);
}

void Logger::LogW(LogLevel level, const std::wstring& msg) {
    Write(level, StringUtils::WStringToUtf8(msg));
}

} // namespace mn