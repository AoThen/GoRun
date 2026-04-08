#pragma once

#include <string>
#include <vector>

namespace mn::StringUtils {

std::string WStringToUtf8(const std::wstring& wstr);
std::wstring Utf8ToWString(const std::string& str);
std::vector<std::wstring> Split(const std::wstring& str, wchar_t delimiter);
std::wstring Trim(const std::wstring& str);
std::wstring ToLower(const std::wstring& str);
bool FuzzyMatch(const std::wstring& text, const std::wstring& query);

// UTF-8 安全截断，按字符数截断而非字节数
std::string TruncateUtf8(const std::string& str, size_t maxChars, const std::string& suffix = "..");

// 获取 UTF-8 字符串的字符数
size_t Utf8CharCount(const std::string& str);

// 拼音相关
std::wstring GetPinyinInitials(const std::wstring& text);
wchar_t GetPinyinInitial(wchar_t ch);
bool PinyinMatch(const std::wstring& text, const std::wstring& query);

// 综合搜索匹配（支持子串、拼音、关键词）
struct MatchResult {
    bool matched = false;
    int score = 0;  // 匹配得分，用于排序
};
MatchResult SearchMatch(const std::wstring& text, const std::wstring& keywords, const std::wstring& query);

} // namespace mn::StringUtils