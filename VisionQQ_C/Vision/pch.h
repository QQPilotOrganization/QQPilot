// pch.h: 这是预编译标头文件。
// 下方列出的文件仅编译一次，提高了将来生成的生成性能。
// 这还将影响 IntelliSense 性能，包括代码完成和许多代码浏览功能。
// 但是，如果此处列出的文件中的任何一个在生成之间有更新，它们全部都将被重新编译。
// 请勿在此处添加要频繁更新的文件，这将使得性能优势无效。

#ifndef PCH_H
#define PCH_H
#define _CRT_SECURE_NO_WARNINGS

// 添加要在此处预编译的标头
// 注意：不要随手加 <iostream>。它会在每个 TU 里塞一个 ios_base::Init 静态对象，
// 把整个 iostream 拽进 DLL（几十 KB），而这个 DLL 不需要往控制台打字。
#include "framework.h"
#include<array>
#include<vector>
#include <limits>
#include <cmath>
#include <iterator>
#include<algorithm>
#include<utility>
#include<memory>
#include"color.h"
#endif //PCH_H
