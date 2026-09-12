#include "pch.h"

extern "C" __declspec(dllexport)
bool hotKey(WORD modifier, WORD vkKey);

extern "C" __declspec(dllexport)
bool press(WORD vkKey);
extern "C" __declspec(dllexport)
WORD getVkKey(const char* key);



