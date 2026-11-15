#pragma once
#ifndef CLOCALIZATION_H
#define CLOCALIZATION_H

// Clocalization：给 C++ 程序用的本地化 C++ DLL (不是C DLL!)。
//
// 这里**没有全局单例**：每个调用方（每个对话框、每个模块）自己 create 一份句柄，
// 各自加载各自的翻译文件，互不干扰。用完 destroy 掉就行（或者用下面的 RAII 包装）。
//
// 翻译表本身由 Rust 那边生成（qqpilot5 / option3 的 localization.rs 写出的
// <locale>.json），这里只负责读。

#include <string>
#include <vector>

#include <windows.h>

/// 不透明句柄：一份翻译表的实例。
struct ClocalizationHandle;

/// 创建一份实例。
///
/// * `fileName` 为 `"auto"`（或空）时，按系统语言找 `<locale>.json`；
///   读不到就退回兜底语言 `zh-CN.json`（和 Rust 那边一致）。
/// * 其它值按文件路径打开。
///
/// 加载失败也返回非空句柄（`clocalization_ready` 为 false），调用方不必到处判空。
extern __declspec(dllexport) ClocalizationHandle* clocalization_create(
    const char* fileName = "auto");

/// 销毁实例；传 `nullptr` 是安全的。
extern __declspec(dllexport) void clocalization_destroy(ClocalizationHandle* handle);

/// 翻译表是否加载成功。
extern __declspec(dllexport) bool clocalization_ready(const ClocalizationHandle* handle);

/// 当前实例实际用的翻译文件名（调试用）。
extern __declspec(dllexport) std::string clocalization_file(const ClocalizationHandle* handle);

/// 取一条翻译。
///
/// 找不到时返回 `L"X<key>"`（和 Rust 的 `get` 一致，一眼能看出漏了哪条）；
/// 句柄为空或未就绪时同样返回 `L"X<key>"`。
extern __declspec(dllexport) std::wstring clocalization_get(
    const ClocalizationHandle* handle, const std::string& key);

/// UTF-8 -> UTF-16。
extern __declspec(dllexport) std::wstring clocalization_utf8_to_wide(const std::string& text);

/// UTF-16 -> UTF-8。
extern __declspec(dllexport) std::string clocalization_wide_to_utf8(const std::wstring& text);

/// 把文案里的 `{0}` `{1}` … 换成实参。
inline std::wstring clocalization_format(
    const std::wstring& pattern, const std::vector<std::wstring>& args)
{
    std::wstring result = pattern;
    for (size_t index = 0; index < args.size(); ++index)
    {
        const std::wstring placeholder = L"{" + std::to_wstring(index) + L"}";
        for (size_t at = result.find(placeholder); at != std::wstring::npos;
             at = result.find(placeholder, at + args[index].size()))
        {
            result.replace(at, placeholder.size(), args[index]);
        }
    }
    return result;
}

/// exe 所在目录（不带结尾反斜杠）；拿不到返回空串。
inline std::wstring clocalization_exe_directory()
{
    wchar_t path[MAX_PATH] = {};
    const DWORD length = GetModuleFileNameW(nullptr, path, MAX_PATH);
    if (length == 0)
    {
        return {};
    }

    std::wstring full(path, length);
    const size_t slash = full.find_last_of(L"\\/");
    return slash == std::wstring::npos ? std::wstring() : full.substr(0, slash);
}

/// 让 Clocalization.dll 在 exe 旁边的 data\ 目录里也能被找到。
///
/// 工程里对 Clocalization.dll 开了延迟加载（DelayLoadDLLs），真正的 LoadLibrary
/// 发生在第一次调用 DLL 函数时，所以在那之前调一下这个函数就能改掉搜索目录。
///
/// 为什么需要：发布目录的结构是 `菜单.exe` 在根上、其它 exe 和 dll 都在 `data\` 里，
/// 而 DLL 的默认搜索只认 exe 所在目录 —— 不指过去的话 `菜单.exe` 会报找不到 DLL。
/// DLL 就在 exe 旁边时不用动，走默认搜索。
inline void clocalization_set_search_path()
{
    const std::wstring exe_dir = clocalization_exe_directory();
    if (exe_dir.empty())
    {
        return;
    }

    const std::wstring data_dir = exe_dir + L"\\data";
    const std::wstring candidate = data_dir + L"\\Clocalization.dll";
    if (GetFileAttributesW(candidate.c_str()) != INVALID_FILE_ATTRIBUTES)
    {
        SetDllDirectoryW(data_dir.c_str());
    }
}

/// RAII 包装：放进对话框类里当成员用，自动销毁。
class Clocalization
{
public:
    explicit Clocalization(const char* fileName = "auto")
        : m_handle(clocalization_create(fileName))
    {
    }

    ~Clocalization() { clocalization_destroy(m_handle); }

    Clocalization(const Clocalization&) = delete;
    Clocalization& operator=(const Clocalization&) = delete;

    bool ready() const { return clocalization_ready(m_handle); }
    std::string file() const { return clocalization_file(m_handle); }
    std::wstring get(const std::string& key) const { return clocalization_get(m_handle, key); }

    std::wstring format(const std::string& key,
                        const std::vector<std::wstring>& args = {}) const
    {
        return clocalization_format(get(key), args);
    }

    ClocalizationHandle* handle() const { return m_handle; }

private:
    ClocalizationHandle* m_handle;
};

#endif // CLOCALIZATION_H
