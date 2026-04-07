#pragma once

#include <string>

namespace mn::PathUtils {

std::wstring GetAppDataPath();
std::wstring GetExePath();
std::wstring GetExeDir();
std::wstring ToAbsolute(const std::wstring& path);
std::wstring ToRelative(const std::wstring& path);
bool Exists(const std::wstring& path);
std::wstring GetParentDir(const std::wstring& path);
std::wstring GetFileName(const std::wstring& path);
bool EnsureDirectory(const std::wstring& path);

} // namespace mn::PathUtils
