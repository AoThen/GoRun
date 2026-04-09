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