#pragma once


#include"pch.h"
namespace Seasoning
{
	int add(const wchar_t* fileSrc, const char* data, uint64_t size, size_t repeat=1);
	int add(const wchar_t* fileSrc, const wchar_t* data, uint64_t size, size_t repeat=1);


}