#include "seasoning.h"

#include"pch.h"
namespace Seasoning
{
	int add(const wchar_t* fileSrc, const char* data, uint64_t size,size_t repeat)
	{
		std::ofstream outFile(fileSrc, std::ios::binary|std::ios::ate|std::ios::app);
		if (!outFile) {
	
			return 1;
		}


		for (size_t i = 0; i < repeat; i++)
		{
			outFile.write(data, size);
		}
	

		return 0;
	}

	int add(const wchar_t* fileSrc, const wchar_t* data, uint64_t size, size_t repeat)
	{
		return add(fileSrc, reinterpret_cast<const char*>(data), size, repeat);
	}
}
