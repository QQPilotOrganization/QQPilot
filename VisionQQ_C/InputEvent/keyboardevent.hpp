#include "pch.h"

extern "C" __declspec(dllexport)
bool hotKey(WORD modifier, WORD vkKey);

extern "C" __declspec(dllexport)
bool press(WORD vkKey);
extern "C" __declspec(dllexport)
WORD getVkKey(const char* key);



extern "C" __declspec(dllexport)
bool hotKeyUp(WORD modifier, WORD vkKey);
extern "C" __declspec(dllexport)
bool hotKeyDown(WORD modifier, WORD vkKey);

extern "C" __declspec(dllexport)
bool keyUp(WORD vkKey);
extern "C" __declspec(dllexport)
bool keyDown(WORD vkKey);			


extern "C" __declspec(dllexport)
bool holdHotKey(WORD modifier, WORD vkKey, float second);
extern "C" __declspec(dllexport)
bool hold(WORD vkKey, float second);