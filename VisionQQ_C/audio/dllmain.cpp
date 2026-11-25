// dllmain.cpp : 定义 DLL 应用程序的入口点。
#include "pch.h"

#define MINIAUDIO_IMPLEMENTATION
#include "..\common\miniaudio.h"


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



constexpr auto maxLength_s = 60;
//播放wav，结束后返回。
extern "C" __declspec(dllexport)
bool playSound2(const char* file)
{
    ma_result result;
    ma_engine engine;
    ma_sound sound;  // 需要自行管理的 sound 对象

    result = ma_engine_init(NULL, &engine);
    if (result != MA_SUCCESS) {
        printf("Failed to initialize audio engine.");
        return false;
    }

    // 用 ma_sound_init_from_file 替代 ma_engine_play_sound
    result = ma_sound_init_from_file(&engine, file, 0, NULL, NULL, &sound);
    
    if (result != MA_SUCCESS) {
        printf("Failed to load sound file.");
        ma_engine_uninit(&engine);
        return false;
    }

    ma_sound_start(&sound);

    unsigned count = 0;
    // 阻塞等待，直到声音播完
    while (ma_sound_at_end(&sound) == 0) {
        Sleep(20);
        count += 20;
        if (count >= (maxLength_s * 1000))
        {
            break;
        }

    }

    // 清理资源
    ma_sound_stop(&sound);
    ma_sound_uninit(&sound);  
    ma_engine_uninit(&engine);
    return true;
}