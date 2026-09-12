#include "pch.h"									   
void setKeyEvent(INPUT& input, WORD vk, bool keyUp = false)

{
    input.type = INPUT_KEYBOARD;
    input.ki.wVk = vk;
    input.ki.wScan = MapVirtualKeyW(vk, MAPVK_VK_TO_VSC);
    input.ki.dwFlags = (keyUp ? KEYEVENTF_KEYUP : 0) | KEYEVENTF_SCANCODE;
}

extern "C" __declspec(dllexport)
WORD getVkKey(const char* key)
{
    if (!key || !*key) return 0;

    // 单字符（字母、数字、符号）
    if (key[1] == '\0') {
        char c = std::toupper(static_cast<unsigned char>(key[0]));
        if (c >= 'A' && c <= 'Z') return static_cast<WORD>(c);
        if (c >= '0' && c <= '9') return static_cast<WORD>(c);
        // 可扩展符号，如 '+' -> VK_OEM_PLUS，但需注意键盘布局
        return static_cast<WORD>(c); // fallback
    }

    // 多字符键名
    struct KeyMap {
        const char* name;
        WORD vk;
    };

    static const KeyMap map[] = {
        {"CTRL",    VK_CONTROL},
        {"CONTROL", VK_CONTROL},
        {"ALT",     VK_MENU},      // ✅ 正确！
        {"SHIFT",   VK_SHIFT},
        {"ENTER",   VK_RETURN},
        {"RETURN",  VK_RETURN},
        {"ESC",     VK_ESCAPE},
        {"ESCAPE",  VK_ESCAPE},
        {"TAB",     VK_TAB},
        {"SPACE",   VK_SPACE},
        {"BACK",    VK_BACK},
        {"BACKSPACE", VK_BACK},
        {"UP",      VK_UP},
        {"DOWN",    VK_DOWN},
        {"LEFT",    VK_LEFT},
        {"RIGHT",   VK_RIGHT},
        {"F1",      VK_F1},
        {"F2",      VK_F2},
        {"F3",      VK_F3},
        {"F4",      VK_F4},
        {"F5",      VK_F5},
        {"F6",      VK_F6},
        {"F7",      VK_F7},
        {"F8",      VK_F8},
        {"F9",      VK_F9},
        {"F10",     VK_F10},
        {"F11",     VK_F11},
        {"F12",     VK_F12},
        {"INSERT",  VK_INSERT},
        {"DELETE",  VK_DELETE},
        {"HOME",    VK_HOME},
        {"END",     VK_END},
        {"PGUP",    VK_PRIOR},
        {"PAGEUP",  VK_PRIOR},
        {"PGDN",    VK_NEXT},
        {"PAGEDOWN",VK_NEXT},
        {"NUM0",    VK_NUMPAD0},
        {"NUM1",    VK_NUMPAD1},
        {"NUM2",    VK_NUMPAD2},
        {"NUM3",    VK_NUMPAD3},
        {"NUM4",    VK_NUMPAD4},
        {"NUM5",    VK_NUMPAD5},
        {"NUM6",    VK_NUMPAD6},
        {"NUM7",    VK_NUMPAD7},
        {"NUM8",    VK_NUMPAD8},
        {"NUM9",    VK_NUMPAD9},
        {"ADD",     VK_ADD},
        {"SUBTRACT",VK_SUBTRACT},
        {"MULTIPLY",VK_MULTIPLY},
        {"DIVIDE",  VK_DIVIDE},
        {"DECIMAL", VK_DECIMAL},
        {nullptr,   0}
    };

    for (int i = 0; map[i].name; ++i) {
        if (std::strcmp(key, map[i].name) == 0 ||
            std::strcmp(key, map[i].name + 1) == 0) // 可选：忽略大小写（见下方）
        {
            return map[i].vk;
        }
    }

    // 可选：尝试忽略大小写（更健壮）
    char upperKey[32] = {};
    size_t len = min(std::strlen(key), sizeof(upperKey) - 1);
    for (size_t i = 0; i < len; ++i) {
        upperKey[i] = std::toupper(static_cast<unsigned char>(key[i]));
    }
    for (int i = 0; map[i].name; ++i) {
        if (std::strcmp(upperKey, map[i].name) == 0) {
            return map[i].vk;
        }
    }

    return 0; // 无效键
}

extern "C" __declspec(dllexport)
bool press(WORD vkKey)
{
    INPUT inputs[2] = {};
    setKeyEvent(inputs[0], vkKey);      // key down
    setKeyEvent(inputs[1], vkKey, true); // key up
    return SendInput(2, inputs, sizeof(INPUT)) == 2;
}

extern "C" __declspec(dllexport)
bool hotKey(WORD modifier, WORD vkKey)
{
    INPUT inputs[4] = {};
    setKeyEvent(inputs[0], modifier);      // mod down
    setKeyEvent(inputs[1], vkKey);         // key down
    setKeyEvent(inputs[2], vkKey, true);   // key up
    setKeyEvent(inputs[3], modifier, true); // mod up
    return SendInput(4, inputs, sizeof(INPUT)) == 4;
}