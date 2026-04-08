#include "StringUtils.h"
#include <algorithm>

#ifdef _WIN32
#include <Windows.h>
// 某些 Windows SDK 版本未定义 LCMAP_PINYIN
#ifndef LCMAP_PINYIN
#define LCMAP_PINYIN 0x02000000
#endif
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

// 拼音首字母表（基于 Unicode 区间）
// 常用汉字范围: 0x4E00 - 0x9FA5
static const wchar_t* PINYIN_TABLE = 
    L"啊芭擦搭蛾发噶哈击喀垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开垃妈拿哦啪期然撒塌挖昔压匝击开垃妈拿哦啪期然撒塌挖昔压匝"
    L"击开";

// 拼音首字母区间表（基于 Unicode 编码范围）
// 格式: {起始Unicode, 结束Unicode, 拼音首字母}
static const struct PinyinRange {
    unsigned short start;
    unsigned short end;
    wchar_t initial;
} PINYIN_RANGES[] = {
    {0xB0A1, 0xB0C4, L'a'},   // 啊
    {0xB0C5, 0xB2C0, L'b'},   // 芭
    {0xB2C1, 0xB4ED, L'c'},   // 擦
    {0xB4EE, 0xB6E9, L'd'},   // 搭
    {0xB6EA, 0xB7A1, L'e'},   // 蛾
    {0xB7A2, 0xB8C0, L'f'},   // 发
    {0xB8C1, 0xB9FD, L'g'},   // 噶
    {0xB9FE, 0xBBF6, L'h'},   // 哈
    {0xBBF7, 0xBFA5, L'j'},   // 击
    {0xBFA6, 0xC0AB, L'k'},   // 咖
    {0xC0AC, 0xC2E7, L'l'},   // 拉
    {0xC2E8, 0xC4C2, L'm'},   // 妈
    {0xC4C3, 0xC5B5, L'n'},   // 拿
    {0xC5B6, 0xC5BD, L'o'},   // 哦
    {0xC5BE, 0xC6D9, L'p'},   // 啪
    {0xC6DA, 0xC8BA, L'q'},   // 期
    {0xC8BB, 0xC8F5, L'r'},   // 然
    {0xC8F6, 0xCBF0, L's'},   // 撒
    {0xCBF1, 0xCDD9, L't'},   // 塌
    {0xCDDA, 0xCEF3, L'w'},   // 挖
    {0xCEF4, 0xD188, L'x'},   // 昔
    {0xD1B9, 0xD4D0, L'y'},   // 压
    {0xD4D1, 0xD7F9, L'z'},   // 匝
};

wchar_t GetPinyinInitial(wchar_t ch) {
    // ASCII 字母直接返回小写
    if (ch >= L'a' && ch <= L'z') return ch;
    if (ch >= L'A' && ch <= L'Z') return static_cast<wchar_t>(ch + 32);
    
    // 数字返回自身
    if (ch >= L'0' && ch <= L'9') return ch;
    
    // 常用汉字范围检查
    if (ch >= 0x4E00 && ch <= 0x9FA5) {
#ifdef _WIN32
        // 使用 Windows API 获取拼音
        wchar_t buffer[20] = {0};
        int result = LCMapStringW(
            MAKELCID(MAKELANGID(LANG_CHINESE_SIMPLIFIED, SUBLANG_CHINESE_SIMPLIFIED), SORT_CHINESE_PRCP),
            LCMAP_PINYIN,
            &ch, 1,
            buffer, 20
        );
        if (result > 0 && buffer[0] >= L'a' && buffer[0] <= L'z') {
            return buffer[0];
        }
#endif
        // 简化的拼音首字母表（基于 GB2312 编码区间）
        // Unicode 转 GB2312 并查表
        unsigned short gb = 0;
#ifdef _WIN32
        char mb[3] = {0};
        int len = WideCharToMultiByte(936, 0, &ch, 1, mb, 3, nullptr, nullptr);
        if (len == 2) {
            gb = (static_cast<unsigned char>(mb[0]) << 8) | static_cast<unsigned char>(mb[1]);
        }
#endif
        if (gb > 0) {
            for (const auto& range : PINYIN_RANGES) {
                if (gb >= range.start && gb <= range.end) {
                    return range.initial;
                }
            }
        }
    }
    
    return L'\0';
}

std::wstring GetPinyinInitials(const std::wstring& text) {
    std::wstring result;
    for (wchar_t ch : text) {
        wchar_t initial = GetPinyinInitial(ch);
        if (initial != L'\0') {
            result += initial;
        }
    }
    return result;
}

bool PinyinMatch(const std::wstring& text, const std::wstring& query) {
    if (query.empty()) return true;
    if (text.empty()) return false;
    
    std::wstring initials = GetPinyinInitials(text);
    std::wstring lowerInitials = ToLower(initials);
    std::wstring lowerQuery = ToLower(query);
    
    return lowerInitials.find(lowerQuery) != std::wstring::npos;
}

MatchResult SearchMatch(const std::wstring& text, const std::wstring& keywords, const std::wstring& query) {
    MatchResult result;
    
    if (query.empty()) {
        result.matched = true;
        result.score = 0;
        return result;
    }
    
    if (text.empty()) {
        result.matched = false;
        return result;
    }
    
    std::wstring lowerText = ToLower(text);
    std::wstring lowerQuery = ToLower(query);
    
    // 1. 名称完全匹配（最高优先级）
    if (lowerText == lowerQuery) {
        result.matched = true;
        result.score = 100;
        return result;
    }
    
    // 2. 名称前缀匹配
    if (lowerText.find(lowerQuery) == 0) {
        result.matched = true;
        result.score = 80;
        return result;
    }
    
    // 3. 名称子串匹配
    size_t pos = lowerText.find(lowerQuery);
    if (pos != std::wstring::npos) {
        result.matched = true;
        result.score = 60 - static_cast<int>(pos);  // 越靠前分数越高
        return result;
    }
    
    // 4. 拼音首字母匹配
    std::wstring initials = GetPinyinInitials(text);
    std::wstring lowerInitials = ToLower(initials);
    if (lowerInitials.find(lowerQuery) != std::wstring::npos) {
        result.matched = true;
        result.score = 40;
        return result;
    }
    
    // 5. 关键词匹配
    if (!keywords.empty()) {
        std::wstring lowerKeywords = ToLower(keywords);
        std::vector<std::wstring> keywordList = Split(lowerKeywords, L',');
        for (const auto& kw : keywordList) {
            std::wstring trimmedKw = Trim(kw);
            if (!trimmedKw.empty() && trimmedKw.find(lowerQuery) != std::wstring::npos) {
                result.matched = true;
                result.score = 30;
                return result;
            }
            // 关键词拼音匹配
            std::wstring kwInitials = GetPinyinInitials(trimmedKw);
            if (!kwInitials.empty() && ToLower(kwInitials).find(lowerQuery) != std::wstring::npos) {
                result.matched = true;
                result.score = 20;
                return result;
            }
        }
    }
    
    result.matched = false;
    return result;
}

size_t Utf8CharCount(const std::string& str) {
    size_t count = 0;
    for (size_t i = 0; i < str.size(); ) {
        unsigned char c = static_cast<unsigned char>(str[i]);
        if ((c & 0x80) == 0) {
            // ASCII 字符
            i += 1;
        } else if ((c & 0xE0) == 0xC0) {
            // 2 字节 UTF-8
            i += 2;
        } else if ((c & 0xF0) == 0xE0) {
            // 3 字节 UTF-8 (中文)
            i += 3;
        } else if ((c & 0xF8) == 0xF0) {
            // 4 字节 UTF-8
            i += 4;
        } else {
            // 无效字节，跳过
            i += 1;
        }
        count++;
    }
    return count;
}

std::string TruncateUtf8(const std::string& str, size_t maxChars, const std::string& suffix) {
    if (str.empty()) return str;
    
    size_t charCount = 0;
    size_t lastValidPos = 0;
    
    for (size_t i = 0; i < str.size(); ) {
        if (charCount >= maxChars) {
            return str.substr(0, lastValidPos) + suffix;
        }
        
        unsigned char c = static_cast<unsigned char>(str[i]);
        size_t charLen = 1;
        
        if ((c & 0x80) == 0) {
            charLen = 1;
        } else if ((c & 0xE0) == 0xC0) {
            charLen = 2;
        } else if ((c & 0xF0) == 0xE0) {
            charLen = 3;
        } else if ((c & 0xF8) == 0xF0) {
            charLen = 4;
        }
        
        lastValidPos = i;
        i += charLen;
        charCount++;
    }
    
    return str;  // 不需要截断
}

} // namespace mn::StringUtils
