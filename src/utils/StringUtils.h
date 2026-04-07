#pragma once

#include <string>
#include <vector>

namespace mn::StringUtils {

std::string WStringToUtf8(const std::wstring& wstr);
std::wstring Utf8ToWString(const std::string& str);
std::vector<std::wstring> Split(const std::wstring& str, wchar_t delimiter);
std::wstring Trim(const std::wstring& str);
std::wstring ToLower(const std::wstring& str);

} // namespace mn::StringUtils