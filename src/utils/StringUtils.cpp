#include "StringUtils.h"
#include <algorithm>

#ifdef _WIN32
#include <Windows.h>
#endif

namespace mn::StringUtils {

std::string WStringToUtf8(const std::wstring& wstr) {
    if (wstr.empty()) return {};
#ifdef _WIN32
    int size = WideCharToMultiByte(CP_UTF8, 0, wstr.c_str(), -1, nullptr, 0, nullptr, nullptr);
    std::string result(size - 1, 0);
    WideCharToMultiByte(CP_UTF8, 0, wstr.c_str(), -1, result.data(), size, nullptr, nullptr);
    return result;
#else
    // Linux/macOS 简化实现
    std::string result;
    for (wchar_t wc : wstr) {
        if (wc < 0x80) {
            result += static_cast<char>(wc);
        } else {
            // 简化处理，实际需要完整 UTF-8 编码
            result += '?';
        }
    }
    return result;
#endif
}

std::wstring Utf8ToWString(const std::string& str) {
    if (str.empty()) return {};
#ifdef _WIN32
    int size = MultiByteToWideChar(CP_UTF8, 0, str.c_str(), -1, nullptr, 0);
    std::wstring result(size - 1, 0);
    MultiByteToWideChar(CP_UTF8, 0, str.c_str(), -1, result.data(), size);
    return result;
#else
    // Linux/macOS 简化实现
    std::wstring result;
    for (char c : str) {
        result += static_cast<wchar_t>(static_cast<unsigned char>(c));
    }
    return result;
#endif
}

std::vector<std::wstring> Split(const std::wstring& str, wchar_t delimiter) {
    std::vector<std::wstring> tokens;
    size_t start = 0;
    size_t end = str.find(delimiter);
    while (end != std::wstring::npos) {
        tokens.push_back(str.substr(start, end - start));
        start = end + 1;
        end = str.find(delimiter, start);
    }
    tokens.push_back(str.substr(start));
    return tokens;
}

std::wstring Trim(const std::wstring& str) {
    auto start = str.find_first_not_of(L" \t\r\n");
    if (start == std::wstring::npos) return {};
    auto end = str.find_last_not_of(L" \t\r\n");
    return str.substr(start, end - start + 1);
}

std::wstring ToLower(const std::wstring& str) {
    std::wstring result = str;
    std::transform(result.begin(), result.end(), result.begin(), ::towlower);
    return result;
}

bool FuzzyMatch(const std::wstring& text, const std::wstring& query) {
    if (query.empty()) return true;
    if (text.empty()) return false;
    
    std::wstring lowerText = ToLower(text);
    std::wstring lowerQuery = ToLower(query);
    
    return lowerText.find(lowerQuery) != std::wstring::npos;
}

} // namespace mn::StringUtils
