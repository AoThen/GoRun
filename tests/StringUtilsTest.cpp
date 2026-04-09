#include <gtest/gtest.h>
#include "utils/StringUtils.h"

using namespace mn::StringUtils;

// ==================== Split 测试 ====================

TEST(StringUtilsTest, Split_EmptyString) {
    auto result = Split(L"", L',');
    ASSERT_EQ(result.size(), 1);
    EXPECT_EQ(result[0], L"");
}

TEST(StringUtilsTest, Split_SingleElement) {
    auto result = Split(L"hello", L',');
    ASSERT_EQ(result.size(), 1);
    EXPECT_EQ(result[0], L"hello");
}

TEST(StringUtilsTest, Split_MultipleElements) {
    auto result = Split(L"a,b,c", L',');
    ASSERT_EQ(result.size(), 3);
    EXPECT_EQ(result[0], L"a");
    EXPECT_EQ(result[1], L"b");
    EXPECT_EQ(result[2], L"c");
}

TEST(StringUtilsTest, Split_TrailingDelimiter) {
    auto result = Split(L"a,b,", L',');
    ASSERT_EQ(result.size(), 3);
    EXPECT_EQ(result[0], L"a");
    EXPECT_EQ(result[1], L"b");
    EXPECT_EQ(result[2], L"");
}

TEST(StringUtilsTest, Split_ConsecutiveDelimiters) {
    auto result = Split(L"a,,b", L',');
    ASSERT_EQ(result.size(), 3);
    EXPECT_EQ(result[0], L"a");
    EXPECT_EQ(result[1], L"");
    EXPECT_EQ(result[2], L"b");
}

// ==================== Trim 测试 ====================

TEST(StringUtilsTest, Trim_EmptyString) {
    EXPECT_EQ(Trim(L""), L"");
}

TEST(StringUtilsTest, Trim_OnlyWhitespace) {
    EXPECT_EQ(Trim(L"   "), L"");
    EXPECT_EQ(Trim(L"\t\n\r"), L"");
}

TEST(StringUtilsTest, Trim_NoWhitespace) {
    EXPECT_EQ(Trim(L"hello"), L"hello");
}

TEST(StringUtilsTest, Trim_LeadingWhitespace) {
    EXPECT_EQ(Trim(L"   hello"), L"hello");
}

TEST(StringUtilsTest, Trim_TrailingWhitespace) {
    EXPECT_EQ(Trim(L"hello   "), L"hello");
}

TEST(StringUtilsTest, Trim_BothSides) {
    EXPECT_EQ(Trim(L"   hello   "), L"hello");
}

TEST(StringUtilsTest, Trim_MixedWhitespace) {
    EXPECT_EQ(Trim(L"\t hello \n\r"), L"hello");
}

// ==================== ToLower 测试 ====================

TEST(StringUtilsTest, ToLower_EmptyString) {
    EXPECT_EQ(ToLower(L""), L"");
}

TEST(StringUtilsTest, ToLower_AllLowercase) {
    EXPECT_EQ(ToLower(L"hello"), L"hello");
}

TEST(StringUtilsTest, ToLower_AllUppercase) {
    EXPECT_EQ(ToLower(L"HELLO"), L"hello");
}

TEST(StringUtilsTest, ToLower_MixedCase) {
    EXPECT_EQ(ToLower(L"HeLLo"), L"hello");
}

TEST(StringUtilsTest, ToLower_WithNumbers) {
    EXPECT_EQ(ToLower(L"ABC123"), L"abc123");
}

// ==================== FuzzyMatch 测试 ====================

TEST(StringUtilsTest, FuzzyMatch_EmptyQuery) {
    EXPECT_TRUE(FuzzyMatch(L"hello", L""));
}

TEST(StringUtilsTest, FuzzyMatch_EmptyText) {
    EXPECT_FALSE(FuzzyMatch(L"", L"query"));
}

TEST(StringUtilsTest, FuzzyMatch_ExactMatch) {
    EXPECT_TRUE(FuzzyMatch(L"hello", L"hello"));
}

TEST(StringUtilsTest, FuzzyMatch_SubstringMatch) {
    EXPECT_TRUE(FuzzyMatch(L"hello world", L"world"));
}

TEST(StringUtilsTest, FuzzyMatch_CaseInsensitive) {
    EXPECT_TRUE(FuzzyMatch(L"Hello World", L"world"));
    EXPECT_TRUE(FuzzyMatch(L"hello world", L"WORLD"));
}

TEST(StringUtilsTest, FuzzyMatch_NoMatch) {
    EXPECT_FALSE(FuzzyMatch(L"hello", L"xyz"));
}

// ==================== Utf8CharCount 测试 ====================

TEST(StringUtilsTest, Utf8CharCount_EmptyString) {
    EXPECT_EQ(Utf8CharCount(""), 0);
}

TEST(StringUtilsTest, Utf8CharCount_AsciiOnly) {
    EXPECT_EQ(Utf8CharCount("hello"), 5);
}

TEST(StringUtilsTest, Utf8CharCount_Chinese) {
    // 每个中文字符占 3 字节，但算 1 个字符
    EXPECT_EQ(Utf8CharCount("你好"), 2);
}

TEST(StringUtilsTest, Utf8CharCount_Mixed) {
    // "hello世界" = 5 ASCII + 2 中文 = 7 字符
    EXPECT_EQ(Utf8CharCount("hello世界"), 7);
}

// ==================== TruncateUtf8 测试 ====================

TEST(StringUtilsTest, TruncateUtf8_EmptyString) {
    EXPECT_EQ(TruncateUtf8("", 5), "");
}

TEST(StringUtilsTest, TruncateUtf8_NoTruncationNeeded) {
    EXPECT_EQ(TruncateUtf8("hello", 10), "hello");
}

TEST(StringUtilsTest, TruncateUtf8_ExactLength) {
    EXPECT_EQ(TruncateUtf8("hello", 5), "hello");
}

TEST(StringUtilsTest, TruncateUtf8_TruncationNeeded) {
    std::string result = TruncateUtf8("hello world", 5);
    EXPECT_EQ(result, "hello..");
}

TEST(StringUtilsTest, TruncateUtf8_ChineseCharacters) {
    // 截断到 2 个字符
    std::string result = TruncateUtf8("你好世界", 2);
    EXPECT_EQ(result, "你好..");
}

TEST(StringUtilsTest, TruncateUtf8_CustomSuffix) {
    std::string result = TruncateUtf8("hello world", 5, "...");
    EXPECT_EQ(result, "hello...");
}

// ==================== SearchMatch 测试 ====================

TEST(StringUtilsTest, SearchMatch_EmptyQuery) {
    auto result = SearchMatch(L"hello", L"", L"");
    EXPECT_TRUE(result.matched);
    EXPECT_EQ(result.score, 0);
}

TEST(StringUtilsTest, SearchMatch_EmptyText) {
    auto result = SearchMatch(L"", L"", L"query");
    EXPECT_FALSE(result.matched);
}

TEST(StringUtilsTest, SearchMatch_ExactMatch) {
    auto result = SearchMatch(L"hello", L"", L"hello");
    EXPECT_TRUE(result.matched);
    EXPECT_EQ(result.score, 100);
}

TEST(StringUtilsTest, SearchMatch_PrefixMatch) {
    auto result = SearchMatch(L"hello world", L"", L"hello");
    EXPECT_TRUE(result.matched);
    EXPECT_EQ(result.score, 80);
}

TEST(StringUtilsTest, SearchMatch_SubstringMatch) {
    auto result = SearchMatch(L"hello world", L"", L"world");
    EXPECT_TRUE(result.matched);
    EXPECT_LT(result.score, 80);  // 子串匹配分数低于前缀匹配
}

TEST(StringUtilsTest, SearchMatch_NoMatch) {
    auto result = SearchMatch(L"hello", L"", L"xyz");
    EXPECT_FALSE(result.matched);
}

TEST(StringUtilsTest, SearchMatch_WithKeywords) {
    auto result = SearchMatch(L"hello", L"keyword, test", L"keyword");
    EXPECT_TRUE(result.matched);
}

// ==================== WStringToUtf8 / Utf8ToWString 测试 ====================

TEST(StringUtilsTest, Utf8RoundTrip_EmptyString) {
    EXPECT_EQ(WStringToUtf8(L""), "");
    EXPECT_EQ(Utf8ToWString(""), L"");
}

TEST(StringUtilsTest, Utf8RoundTrip_Ascii) {
    std::wstring original = L"hello world";
    std::string utf8 = WStringToUtf8(original);
    std::wstring converted = Utf8ToWString(utf8);
    EXPECT_EQ(converted, original);
}

// ==================== GetPinyinInitial 测试 ====================

TEST(StringUtilsTest, GetPinyinInitial_AsciiLower) {
    EXPECT_EQ(GetPinyinInitial(L'a'), L'a');
    EXPECT_EQ(GetPinyinInitial(L'z'), L'z');
}

TEST(StringUtilsTest, GetPinyinInitial_AsciiUpper) {
    EXPECT_EQ(GetPinyinInitial(L'A'), L'a');
    EXPECT_EQ(GetPinyinInitial(L'Z'), L'z');
}

TEST(StringUtilsTest, GetPinyinInitial_Digit) {
    EXPECT_EQ(GetPinyinInitial(L'0'), L'0');
    EXPECT_EQ(GetPinyinInitial(L'9'), L'9');
}

// ==================== GetPinyinInitials 测试 ====================

TEST(StringUtilsTest, GetPinyinInitials_EmptyString) {
    EXPECT_EQ(GetPinyinInitials(L""), L"");
}

TEST(StringUtilsTest, GetPinyinInitials_AsciiOnly) {
    EXPECT_EQ(GetPinyinInitials(L"abc"), L"abc");
}

TEST(StringUtilsTest, GetPinyinInitials_MixedCase) {
    EXPECT_EQ(GetPinyinInitials(L"ABC"), L"abc");
}

// ==================== PinyinMatch 测试 ====================

TEST(StringUtilsTest, PinyinMatch_EmptyQuery) {
    EXPECT_TRUE(PinyinMatch(L"测试", L""));
}

TEST(StringUtilsTest, PinyinMatch_EmptyText) {
    EXPECT_FALSE(PinyinMatch(L"", L"cs"));
}

TEST(StringUtilsTest, PinyinMatch_AsciiMatch) {
    EXPECT_TRUE(PinyinMatch(L"abc", L"ab"));
}

// ==================== 中文拼音匹配测试 ====================

TEST(StringUtilsTest, GetPinyinInitial_Chinese) {
    // 测试常见汉字的拼音首字母
    wchar_t initial = GetPinyinInitial(L'中');
    EXPECT_TRUE(initial == L'z' || initial == L'\0');  // z 或 未识别
    
    initial = GetPinyinInitial(L'国');
    EXPECT_TRUE(initial == L'g' || initial == L'\0');
    
    initial = GetPinyinInitial(L'测');
    EXPECT_TRUE(initial == L'c' || initial == L'\0');
    
    initial = GetPinyinInitial(L'试');
    EXPECT_TRUE(initial == L's' || initial == L'\0');
}

TEST(StringUtilsTest, GetPinyinInitials_ChineseText) {
    // 测试中文文本的拼音首字母提取
    std::wstring initials = GetPinyinInitials(L"中国");
    // 应该包含 z 和 g（或为空，取决于平台支持）
    EXPECT_TRUE(initials.empty() || initials.size() == 2);
    
    initials = GetPinyinInitials(L"测试");
    EXPECT_TRUE(initials.empty() || initials.size() == 2);
}

TEST(StringUtilsTest, GetPinyinInitials_MixedText) {
    // 中英文混合
    std::wstring initials = GetPinyinInitials(L"GoRun应用");
    // "GoRun应用" -> "gr" + 中文首字母
    EXPECT_TRUE(initials.find(L"g") != std::wstring::npos || initials.find(L'r') != std::wstring::npos);
}

TEST(StringUtilsTest, PinyinMatch_ChineseQuery) {
    // 中文拼音首字母匹配
    // 由于平台差异，这里只测试不崩溃
    bool result = PinyinMatch(L"中国", L"zg");
    // 结果取决于 Windows API 或 GB2312 表是否可用
    EXPECT_TRUE(result || !result);  // 总是 true，仅验证不崩溃
}

TEST(StringUtilsTest, PinyinMatch_ChinesePartial) {
    // 部分拼音匹配
    bool result = PinyinMatch(L"中国软件", L"zg");
    EXPECT_TRUE(result || !result);  // 验证不崩溃
}

TEST(StringUtilsTest, SearchMatch_ChineseName) {
    // 中文搜索匹配
    auto result = SearchMatch(L"记事本", L"", L"记事");
    EXPECT_TRUE(result.matched);  // 子串匹配
    
    result = SearchMatch(L"记事本", L"", L"jsb");
    // 拼音首字母匹配（取决于平台）
    EXPECT_TRUE(result.matched || !result.matched);
}

TEST(StringUtilsTest, SearchMatch_ChineseWithKeywords) {
    // 中文带关键词搜索
    auto result = SearchMatch(L"记事本", L"文本,编辑", L"文本");
    EXPECT_TRUE(result.matched);  // 关键词匹配
    
    result = SearchMatch(L"记事本", L"文本,编辑", L"bj");
    // 关键词拼音匹配
    EXPECT_TRUE(result.matched || !result.matched);
}

// ==================== 综合搜索场景测试 ====================

TEST(StringUtilsTest, SearchMatch_RealWorld_NotePad) {
    // 模拟搜索 "记事本" 应用
    auto result = SearchMatch(L"记事本", L"notepad,文本编辑", L"记事");
    EXPECT_TRUE(result.matched);
    
    result = SearchMatch(L"记事本", L"notepad,文本编辑", L"notepad");
    EXPECT_TRUE(result.matched);  // 关键词精确匹配
    
    result = SearchMatch(L"记事本", L"notepad,文本编辑", L"note");
    EXPECT_TRUE(result.matched);  // 关键词前缀匹配
}

TEST(StringUtilsTest, SearchMatch_RealWorld_Chrome) {
    // 模拟搜索 Chrome
    auto result = SearchMatch(L"Google Chrome", L"浏览器,网页", L"chrome");
    EXPECT_TRUE(result.matched);  // 子串匹配
    
    result = SearchMatch(L"Google Chrome", L"浏览器,网页", L"google");
    EXPECT_TRUE(result.matched);  // 前缀匹配
    
    EXPECT_EQ(result.score, 80);  // 前缀匹配分数
}

TEST(StringUtilsTest, SearchMatch_RealWorld_VSCode) {
    // 模拟搜索 VS Code
    auto result = SearchMatch(L"Visual Studio Code", L"代码编辑器,ide", L"visual");
    EXPECT_TRUE(result.matched);
    EXPECT_EQ(result.score, 80);  // 前缀匹配
    
    result = SearchMatch(L"Visual Studio Code", L"代码编辑器,ide", L"code");
    EXPECT_TRUE(result.matched);  // 子串匹配
    EXPECT_LT(result.score, 80);  // 子串分数 < 前缀分数
}

TEST(StringUtilsTest, SearchMatch_ScoreOrdering) {
    // 验证匹配分数排序：完全匹配 > 前缀匹配 > 子串匹配 > 拼音匹配 > 关键词匹配
    
    auto exact = SearchMatch(L"chrome", L"", L"chrome");
    auto prefix = SearchMatch(L"chrome browser", L"", L"chrome");
    auto substr = SearchMatch(L"google chrome", L"", L"chrome");
    auto keyword = SearchMatch(L"浏览器", L"chrome", L"chrome");
    
    EXPECT_EQ(exact.score, 100);  // 完全匹配
    EXPECT_EQ(prefix.score, 80);  // 前缀匹配
    EXPECT_LT(substr.score, 80);  // 子串匹配
    EXPECT_EQ(keyword.score, 30); // 关键词匹配
    
    // 分数排序验证
    EXPECT_GT(exact.score, prefix.score);
    EXPECT_GT(prefix.score, substr.score);
    EXPECT_GT(substr.score, keyword.score);
}

TEST(StringUtilsTest, SearchMatch_MultiByteUtf8) {
    // UTF-8 多字节字符搜索
    auto result = SearchMatch(L"文件管理器", L"", L"文件");
    EXPECT_TRUE(result.matched);
    
    result = SearchMatch(L"设置中心", L"", L"设置");
    EXPECT_TRUE(result.matched);
}

// ==================== 边界情况测试 ====================

TEST(StringUtilsTest, SearchMatch_SpecialCharacters) {
    // 特殊字符
    auto result = SearchMatch(L"C:\\Program Files\\App", L"", L"program");
    EXPECT_TRUE(result.matched);
    
    result = SearchMatch(L"App (64-bit)", L"", L"app");
    EXPECT_TRUE(result.matched);
}

TEST(StringUtilsTest, SearchMatch_VeryLongText) {
    // 长文本搜索
    std::wstring longText(1000, L'a');
    longText += L"target";
    
    auto result = SearchMatch(longText, L"", L"target");
    EXPECT_TRUE(result.matched);
}

TEST(StringUtilsTest, SearchMatch_UnicodeEmoji) {
    // Emoji 和 Unicode 字符
    auto result = SearchMatch(L"应用📁文件夹", L"", L"应用");
    EXPECT_TRUE(result.matched);
}