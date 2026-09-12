// dllmain.cpp : 定义 DLL 应用程序的入口点。
#include "pch.h"
#include "keyboardevent.hpp"
#include "mouseEvent.hpp"

BOOL APIENTRY DllMain( HMODULE hModule,
                       DWORD  ul_reason_for_call,
                       LPVOID lpReserved
                     )
{
    switch (ul_reason_for_call)
    {
    case DLL_PROCESS_ATTACH:
    case DLL_THREAD_ATTACH:
    case DLL_THREAD_DETACH:
    case DLL_PROCESS_DETACH:
        break;
    }
    return TRUE;
}

extern "C" __declspec(dllexport)
bool Mousegoto(unsigned x, unsigned y);

extern "C" __declspec(dllexport)
bool Lclick(unsigned x, unsigned y);


extern "C" __declspec(dllexport)
bool dragFromTo(unsigned x1, unsigned y1, unsigned x2, unsigned y2, float durationSeconds);



extern "C" __declspec(dllexport)
bool scrollUp(int delta );

extern "C"       __declspec(dllexport)
bool SmoothMousegoto(unsigned x, unsigned y);

__declspec(dllexport)
bool scrollDown(int delta );

extern "C" __declspec(dllexport)
bool scrollLeft(int delta);

extern "C" __declspec(dllexport)
bool scrollRight(int delta);


extern "C" __declspec(dllexport)
WORD getVkKey(const char* key);

extern "C" __declspec(dllexport)
bool copy() { return hotKey(VK_CONTROL, 'C'); }
extern "C" __declspec(dllexport)
bool paste() { return hotKey(VK_CONTROL, 'V'); }
extern "C" __declspec(dllexport)
bool selectAll() { return hotKey(VK_CONTROL, 'A'); }
extern "C" __declspec(dllexport)
bool undo() { return hotKey(VK_CONTROL, 'Z'); }

extern "C" __declspec(dllexport)
bool hotKey(WORD modifier, WORD vkKey);

extern "C" __declspec(dllexport)
bool press(WORD vkKey);
extern "C" __declspec(dllexport)
bool LmouseDown();
extern "C" __declspec(dllexport)
bool LmouseUp();

extern "C" __declspec(dllexport)
bool RmouseUp();
extern "C" __declspec(dllexport)
bool RmouseDown();
extern "C" __declspec(dllexport)
bool Rclick(unsigned x, unsigned y);

extern "C" __declspec(dllexport)
void DPIAwarenessPrologue2()
{
    SetProcessDpiAwareness(PROCESS_SYSTEM_DPI_AWARE);
}

#include <combaseapi.h> // for CoCreateInstance, etc.

extern "C" __declspec(dllexport)
bool DPIAwarenessPrologue()
{
    // 尝试 Windows 8.1+ 方法
    HMODULE shcore = LoadLibraryW(L"shcore.dll");
    if (shcore) {
        typedef HRESULT(WINAPI* SetProcessDpiAwarenessFunc)(PROCESS_DPI_AWARENESS);
        auto func = (SetProcessDpiAwarenessFunc)GetProcAddress(shcore, "SetProcessDpiAwareness");
        if (func) {
            HRESULT hr = func(PROCESS_SYSTEM_DPI_AWARE);
            FreeLibrary(shcore);
            return SUCCEEDED(hr);
        }
        FreeLibrary(shcore);
    }

    // 回退到 Vista~Win8 方法（仅系统 DPI 感知）
    typedef BOOL(WINAPI* SetProcessDPIAwareFunc)();
    HMODULE user32 = GetModuleHandleW(L"user32.dll");
    if (user32) {
        auto func = (SetProcessDPIAwareFunc)GetProcAddress(user32, "SetProcessDPIAware");
        if (func) {
            return func() != FALSE;
        }
    }

    return false; // 不支持
}