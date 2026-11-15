// Clocalization：给 C++ 程序用的本地化 DLL。
// 翻译表由 Rust 那边生成（qqpilot5 / option3 的 localization.rs 写 <locale>.json）。

#include "localization.h"

#include <windows.h>

#include <cstdio>
#include <fstream>
#include <sstream>
#include <utility>

#include <json.hpp>
#include <utf8.h>

/// 一份翻译表实例。
struct ClocalizationHandle
{
    nlohmann::json table;
    bool ready = false;
    std::string file;
};

namespace
{
    std::string to_utf8(const std::wstring& text)
    {
        std::vector<char> buffer;
        utf8::utf16to8(text.c_str(), text.c_str() + text.length(),
                       std::back_inserter(buffer));
        return std::string(buffer.begin(), buffer.end());
    }

    std::wstring to_wide(const std::string& text)
    {
        std::vector<wchar_t> buffer;
        utf8::utf8to16(text.c_str(), text.c_str() + text.length(),
                       std::back_inserter(buffer));
        return std::wstring(buffer.begin(), buffer.end());
    }

    /// 读一个 JSON 翻译文件；不存在、解析失败、顶层不是对象都算失败。
    bool load_table(const std::string& path, nlohmann::json& table)
    {
        std::ifstream input(path, std::ios::binary);
        if (!input)
        {
            return false;
        }

        std::stringstream content;
        content << input.rdbuf();
        std::string text = content.str();

        // 有些编辑器存 UTF-8 时会带 BOM，直接喂给 nlohmann 会解析失败。
        if (text.size() >= 3 && static_cast<unsigned char>(text[0]) == 0xEF
            && static_cast<unsigned char>(text[1]) == 0xBB
            && static_cast<unsigned char>(text[2]) == 0xBF)
        {
            text.erase(0, 3);
        }
        if (text.empty())
        {
            return false;
        }

        nlohmann::json parsed = nlohmann::json::parse(text, nullptr, false);
        if (parsed.is_discarded() || !parsed.is_object() || parsed.empty())
        {
            return false;
        }

        table = std::move(parsed);
        return true;
    }

    /// 系统语言（形如 zh-CN）；拿不到返回空串。
    std::string system_locale()
    {
        WCHAR name[LOCALE_NAME_MAX_LENGTH] = {};
        const int length = GetUserDefaultLocaleName(name, LOCALE_NAME_MAX_LENGTH);
        if (length <= 0)
        {
            return {};
        }
        return to_utf8(std::wstring(name, static_cast<size_t>(length) - 1));
    }
}

extern __declspec(dllexport) ClocalizationHandle* clocalization_create(const char* fileName)
{
    auto* handle = new ClocalizationHandle();

    const std::string requested = (fileName == nullptr) ? "" : fileName;
    if (requested.empty() || requested == "auto")
    {
        const std::string locale = system_locale();
        if (!locale.empty() && load_table(locale + ".json", handle->table))
        {
            handle->file = locale + ".json";
        }
        // 系统语言没有对应的表：退回兜底语言，别让整个界面变成 X<key>。
        else if (load_table("zh-CN.json", handle->table))
        {
            handle->file = "zh-CN.json";
        }
    }
    else if (load_table(requested, handle->table))
    {
        handle->file = requested;
    }

    handle->ready = !handle->file.empty();
    return handle;
}

extern __declspec(dllexport) void clocalization_destroy(ClocalizationHandle* handle)
{
    delete handle;
}

extern __declspec(dllexport) bool clocalization_ready(const ClocalizationHandle* handle)
{
    return handle != nullptr && handle->ready;
}

extern __declspec(dllexport) std::string clocalization_file(const ClocalizationHandle* handle)
{
    return handle == nullptr ? std::string() : handle->file;
}

extern __declspec(dllexport) std::wstring clocalization_get(
    const ClocalizationHandle* handle, const std::string& key)
{
    if (handle != nullptr && handle->ready)
    {
        const auto found = handle->table.find(key);
        if (found != handle->table.end() && found->is_string())
        {
            return to_wide(found->get<std::string>());
        }

        // 找不到就提示一句（GUI 程序里 stderr 看不到，调试器里能看到）。
        const std::string message = "未找到 " + key + " 的翻译（" + handle->file + "）\n";
        OutputDebugStringA(message.c_str());
        std::fputs(message.c_str(), stderr);
    }
    else
    {
        const std::string message = "未初始化，无法翻译 " + key + "\n";
        OutputDebugStringA(message.c_str());
        std::fputs(message.c_str(), stderr);
    }

    return L"X" + to_wide(key);
}

extern __declspec(dllexport) std::wstring clocalization_utf8_to_wide(const std::string& text)
{
    return to_wide(text);
}

extern __declspec(dllexport) std::string clocalization_wide_to_utf8(const std::wstring& text)
{
    return to_utf8(text);
}
