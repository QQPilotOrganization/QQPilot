// clocalc.cpp: Clocalization 的测试。
//
// 这里测的是 Clocalization 目录里的**同一份** localization.cpp（本工程里的
// localization.cpp 只是 #include 了它），所以不会有"测试过了但 DLL 里是旧代码"的问题。
//
// 不用 assert：Release 里 NDEBUG 一开 assert 就没了，这里自己记失败数。

#include <windows.h>

#include <cstdio>
#include <string>
#include <vector>

#include "localization.h"

namespace
{
    int g_checks = 0;
    int g_failures = 0;

    /// 控制台按 UTF-8 输出，wide 字符串先转成 UTF-8。
    void report(const char* name, const std::wstring& detail = L"")
    {
        std::printf("  [失败] %s", name);
        if (!detail.empty())
        {
            std::printf("：%s", clocalization_wide_to_utf8(detail).c_str());
        }
        std::printf("\n");
    }

    void check(bool condition, const char* name, const std::wstring& detail = L"")
    {
        ++g_checks;
        if (condition)
        {
            return;
        }
        ++g_failures;
        report(name, detail);
    }

    void check_text(const std::wstring& actual, const std::wstring& expected, const char* name)
    {
        check(actual == expected, name, L"实际是 [" + actual + L"]");
    }

    /// 显式文件、正常取值。
    void test_explicit_file()
    {
        ClocalizationHandle* handle = clocalization_create("test.json");
        check(clocalization_ready(handle), "显式文件能加载");
        check_text(clocalization_get(handle, "chinese"), L"中文测试", "中文键");
        check_text(clocalization_get(handle, "ascii"), L"Ascii Test", "英文键");
        check_text(clocalization_get(handle, "mixed"),
                   L"你好;++ or '1'=='1' -- \" \" 😒", "混合/特殊字符键");
        check_text(clocalization_get(handle, "no-such-key"), L"Xno-such-key", "缺键返回 X<key>");
        check(clocalization_file(handle) == "test.json", "记录实际用的文件");
        clocalization_destroy(handle);
    }

    /// 每个实例读各自的文件 —— 这正是去掉单例的目的。
    void test_instances_are_independent()
    {
        ClocalizationHandle* first = clocalization_create("test.json");
        ClocalizationHandle* second = clocalization_create("other.json");

        check_text(clocalization_get(first, "chinese"), L"中文测试", "第一个实例");
        check_text(clocalization_get(second, "chinese"), L"另一个文件", "第二个实例");

        // 销毁其中一个，另一个必须照常工作（有全局状态的话这里就串了）。
        clocalization_destroy(first);
        check_text(clocalization_get(second, "chinese"), L"另一个文件", "销毁第一个后第二个仍可用");
        check_text(clocalization_get(second, "onlyInOther"), L"只有这份文件有", "第二个实例独有键");

        clocalization_destroy(second);
    }

    /// 同一份文件可以同时被多个实例加载。
    void test_same_file_twice()
    {
        ClocalizationHandle* first = clocalization_create("test.json");
        ClocalizationHandle* second = clocalization_create("test.json");
        check_text(clocalization_get(first, "chinese"), L"中文测试", "同文件实例一");
        check_text(clocalization_get(second, "chinese"), L"中文测试", "同文件实例二");
        clocalization_destroy(second);
        check_text(clocalization_get(first, "chinese"), L"中文测试", "销毁另一个后仍可用");
        clocalization_destroy(first);
    }

    /// auto：系统语言没有对应的表就要退回 zh-CN.json，不能整份界面都是 X<key>。
    void test_auto_locale()
    {
        ClocalizationHandle* handle = clocalization_create("auto");
        check(clocalization_ready(handle), "auto 能加载");
        check_text(clocalization_get(handle, "chinese"), L"中文测试", "auto 取到兜底语言");
        clocalization_destroy(handle);

        // 不传参数也应该按 auto 处理
        ClocalizationHandle* defaulted = clocalization_create();
        check(clocalization_ready(defaulted), "不传参数按 auto 处理");
        clocalization_destroy(defaulted);
    }

    /// 文件不存在：句柄仍然可用，但 ready 为 false，取值为 X<key>。
    void test_missing_file()
    {
        ClocalizationHandle* handle = clocalization_create("no-such-file.json");
        check(!clocalization_ready(handle), "文件不存在时 ready() 为 false");
        check_text(clocalization_get(handle, "chinese"), L"Xchinese", "未就绪时返回 X<key>");
        clocalization_destroy(handle);
    }

    /// 空句柄不能崩。
    void test_null_handle()
    {
        check(!clocalization_ready(nullptr), "空句柄 ready() 为 false");
        check_text(clocalization_get(nullptr, "chinese"), L"Xchinese", "空句柄返回 X<key>");
        check(clocalization_file(nullptr).empty(), "空句柄没有文件名");
        clocalization_destroy(nullptr);
        check(true, "destroy(nullptr) 不崩溃");
    }

    /// 带 UTF-8 BOM 的文件也要能读。
    void test_bom_file()
    {
        ClocalizationHandle* handle = clocalization_create("bom.json");
        check(clocalization_ready(handle), "带 BOM 的文件能加载");
        check_text(clocalization_get(handle, "chinese"), L"中文测试", "BOM 文件里的键");
        clocalization_destroy(handle);
    }

    /// 空文件 / 顶层不是对象，都算加载失败。
    void test_broken_file()
    {
        ClocalizationHandle* empty = clocalization_create("empty.json");
        check(!clocalization_ready(empty), "空文件不算加载成功");
        clocalization_destroy(empty);

        ClocalizationHandle* broken = clocalization_create("broken.json");
        check(!clocalization_ready(broken), "非法 JSON 不算加载成功");
        clocalization_destroy(broken);
    }

    /// UTF-8 <-> UTF-16。
    void test_conversions()
    {
        const std::string utf8 = "你好 😒 ascii";
        check_text(clocalization_utf8_to_wide(utf8), L"你好 😒 ascii", "UTF-8 -> UTF-16");
        check(clocalization_wide_to_utf8(L"你好 😒 ascii") == utf8, "UTF-16 -> UTF-8");
        check_text(clocalization_utf8_to_wide(""), L"", "空串转换");
    }

    /// 带占位符的文案。
    void test_format()
    {
        check_text(clocalization_format(L"成功复制：{0}张，失败：{1}张", {L"3", L"1"}),
                   L"成功复制：3张，失败：1张", "占位符替换");
        check_text(clocalization_format(L"没有占位符", {}), L"没有占位符", "没有占位符");
        check_text(clocalization_format(L"{0}{0}", {L"重复"}), L"重复重复", "同一个占位符出现两次");
    }

    /// RAII 包装。
    void test_wrapper()
    {
        Clocalization localization("test.json");
        check(localization.ready(), "包装类 ready");
        check_text(localization.get("chinese"), L"中文测试", "包装类 get");
        check_text(localization.format("chinese"), L"中文测试", "包装类 format（无占位符）");

        Clocalization missing("no-such-file.json");
        check(!missing.ready(), "包装类未就绪");
    }
}

int main()
{
    SetConsoleOutputCP(CP_UTF8);

    // 测试数据在 localization-test 目录（不污染输出目录）,请放到Debug中；
    // 在 Visual Studio 里跑时工作目录是工程目录，直接就能找到。
    if (GetFileAttributesW(L"localization-test") != INVALID_FILE_ATTRIBUTES)
    {
        SetCurrentDirectoryW(L"localization-test");
    }
    else if (GetFileAttributesW(L"test.json") == INVALID_FILE_ATTRIBUTES)
    {
        std::printf("找不到测试用的 json，请在输出目录或工程目录下运行。\n");
        return 1;
    }

    std::printf("Clocalization 测试\n");

    test_explicit_file();
    test_instances_are_independent();
    test_same_file_twice();
    test_auto_locale();
    test_missing_file();
    test_null_handle();
    test_bom_file();
    test_broken_file();
    test_conversions();
    test_format();
    test_wrapper();

    if (g_failures != 0)
    {
        std::printf("失败 %d / %d\n", g_failures, g_checks);
        return 1;
    }

    std::printf("Pass.（%d 项检查）\n", g_checks);
    return 0;
}
