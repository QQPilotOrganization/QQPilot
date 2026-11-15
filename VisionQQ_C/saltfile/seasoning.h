#pragma once
#include<cstdio>
#include<cstdint>
#include<fstream>

namespace Seasoning
{
	int add(const char* fileSrc, const char* data, uint64_t size, size_t repeat=1);
	int add(const char* fileSrc, const wchar_t* data, uint64_t size, size_t repeat=1);


}