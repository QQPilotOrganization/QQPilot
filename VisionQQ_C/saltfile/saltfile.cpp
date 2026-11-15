// saltfile.cpp : 此文件包含 "main" 函数。程序执行将在此处开始并结束。
//

#include <iostream>
#include <random>
#include<string>
#include<cstring>
#include "seasoning.h"
#include <print>
static void addOne(const char* fileName)
{
	time_t now = time(NULL);
	char buffer[26];
	errno_t err = ctime_s(buffer, sizeof(buffer), &now);
	std::string salt;
	std::random_device rd;
	std::mt19937 mt19937(rd());
	std::uniform_int_distribution<uint64_t> distrib((std::numeric_limits<uint64_t>::min)(), (std::numeric_limits<uint64_t>::max)());
	salt += buffer;
	salt += std::to_string(now);
	salt += std::to_string(rand());
	salt += std::to_string(distrib(mt19937));
	Seasoning::add(fileName, salt.c_str(), strlen(salt.c_str()), 1);
}
int main(int argc, char** argv)
{
	int i = 0;
	if (argc == 1)
	{
		goto printHelp;
	}
	for (i = 1; i < argc; i++)
	{
		if (std::strcmp(argv[i], "-h") == 0)
		{
		printHelp:
			std::print("在文件后增加一串时间相关的后缀以修改散列\n");
			std::print("用法{} file1 <file2> ...\n", argv[0]);
			return 0;
		}
		std::print("{}\n", argv[i]);
		addOne(argv[i]);
	}
}

// 运行程序: Ctrl + F5 或调试 >“开始执行(不调试)”菜单
// 调试程序: F5 或调试 >“开始调试”菜单

// 入门使用技巧: 
//   1. 使用解决方案资源管理器窗口添加/管理文件
//   2. 使用团队资源管理器窗口连接到源代码管理
//   3. 使用输出窗口查看生成输出和其他消息
//   4. 使用错误列表窗口查看错误
//   5. 转到“项目”>“添加新项”以创建新的代码文件，或转到“项目”>“添加现有项”以将现有代码文件添加到项目
//   6. 将来，若要再次打开此项目，请转到“文件”>“打开”>“项目”并选择 .sln 文件
