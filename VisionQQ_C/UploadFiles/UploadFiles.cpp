// UploadFiles.cpp : 把一批文件传到 QQ 聊天窗口，并点"发送"。
//
// 全部复用 VisionQQ_C 里已有的 DLL：
//   ScreenCapture.dll  —— 截全屏到 screenshot.png
//   Vision.dll         —— 在截图里找 file.png（那个"文件"图标）
//   InputEvent.dll     —— 设置 DPI 感知 + 鼠标点击
//   uploadFile.dll     —— 往上传对话框里填路径并点"打开"
//
// 流程：
//   1. 声明 DPI 感知，读 config.ini 的 scale / width / height
//   2. 自行截图
//   3. 定位 file.png，算出图标中心
//   4. 点开上传对话框，等 1 秒
//   5. 一次上传所有文件（填进对话框并点"打开"），等 1 秒
//   6. 点 readyToSend 对应点，退出
//
// 用法：
//   UploadFiles.exe <目录或文件> [更多文件...] [选项]
//   选项见 --help。

#include <Windows.h>

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <thread>
#include <vector>

#include "inicpp.hpp"
#include "ot.h"

namespace fs = std::filesystem;

// ---- 与既有代码一致的常量 -----------------------------------------------

constexpr int DefaultSizeW = 1280;
constexpr int DefaultSizeH = 720;

RECT UPLOAD_FILE_POSSIBLE_BBOX_ABSOLUTE_SIZE = {241, 513, 543, 562};

// 文件传完之后"发送"按钮的位置（屏幕物理像素）。
constexpr int readyToSendX = 945;
constexpr int readyToSendY = 763;

// 截图/模板的固定文件名：DLL 内部写死，改不了。
const char* const SCREENSHOT_FILE = "screenshot.png";
const char* const TEMPLATE_FILE = "file.png";

// 上传对话框的标题里一定含有这三个字（uploadFile.dll 也是按这个找窗口的）。
const wchar_t* const DIALOG_TITLE_KEYWORD = L"请选择";

// 模板匹配容差：与 qqpilot5 里找 copy.png 用的值一致。
constexpr int DefaultTolerance = 30;

// ---- 小工具 -------------------------------------------------------------

static void sleep_ms(int milliseconds)
{
	std::this_thread::sleep_for(std::chrono::milliseconds(milliseconds));
}

static void log_line(const std::string& text)
{
	std::cout << text << std::endl;
}

static int fail(const std::string& text, int code)
{
	std::cerr << "错误: " << text << std::endl;
	return code;
}

// config.ini 里的缩放和窗口尺寸；读不到就用默认值。
struct Settings
{
	float scale = 1.0f;
	int width = DefaultSizeW;
	int height = DefaultSizeH;
};

static Settings read_settings()
{
	Settings settings;
	if (not fs::is_regular_file("config.ini"))
	{
		std::cerr << "警告: 当前目录没有 config.ini，scale 用默认的 1.0" << std::endl;
		return settings;
	}

	try
	{
		inicpp::IniManager ini("config.ini");
		settings.scale = ini["general"]["scale"].get<float>();
		settings.width = ini["general"]["width"].get<int>();
		settings.height = ini["general"]["height"].get<int>();
	}
	catch (const std::exception& e)
	{
		std::cerr << "警告: 读 config.ini 失败（" << e.what() << "），scale 用默认的 1.0" << std::endl;
		settings.scale = 1.0f;
	}

	if (settings.scale <= 0.0f)
	{
		settings.scale = 1.0f;
	}
	return settings;
}

static fs::path resolve_template()
{
	wchar_t exe_path[MAX_PATH]{};
	if (GetModuleFileNameW(nullptr, exe_path, MAX_PATH) > 0)
	{
		const fs::path beside_exe = fs::path(exe_path).parent_path() / TEMPLATE_FILE;
		if (fs::is_regular_file(beside_exe))
		{
			return beside_exe;
		}
	}

	const fs::path in_cwd = fs::path(TEMPLATE_FILE);
	if (fs::is_regular_file(in_cwd))
	{
		return in_cwd;
	}
	return {};
}

// 从 PNG 的 IHDR 里读宽高（Vision.dll 用 stb 加载，模板一定是 PNG）。
static bool png_size(const fs::path& path, int& width, int& height)
{
	std::ifstream file(path, std::ios::binary);
	unsigned char header[24]{};
	if (not file.read(reinterpret_cast<char*>(header), sizeof(header)))
	{
		return false;
	}

	static const unsigned char signature[8] = {0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A};
	if (std::memcmp(header, signature, sizeof(signature)) != 0)
	{
		return false;
	}

	width = (header[16] << 24) | (header[17] << 16) | (header[18] << 8) | header[19];
	height = (header[20] << 24) | (header[21] << 16) | (header[22] << 8) | header[23];
	return width > 0 && height > 0;
}

// 上传对话框是否已经弹出来（标题含"请选择"）。
static bool upload_dialog_visible()
{
	bool found = false;
	EnumWindows(
		[](HWND hwnd, LPARAM lparam) -> BOOL
		{
			auto* result = reinterpret_cast<bool*>(lparam);
			if (not IsWindowVisible(hwnd))
			{
				return TRUE;
			}

			wchar_t title[512]{};
			const int length = GetWindowTextW(hwnd, title, static_cast<int>(std::size(title)));
			if (length <= 0)
			{
				return TRUE;
			}
			if (std::wstring(title, length).find(DIALOG_TITLE_KEYWORD) == std::wstring::npos)
			{
				return TRUE;
			}

			*result = true;
			return FALSE;
		},
		reinterpret_cast<LPARAM>(&found));
	return found;
}

// 等对话框出现 / 消失；超时返回 false。
static bool wait_for_dialog(bool should_exist, int timeout_ms)
{
	const auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(timeout_ms);
	while (std::chrono::steady_clock::now() < deadline)
	{
		if (upload_dialog_visible() == should_exist)
		{
			return true;
		}
		sleep_ms(100);
	}
	return false;
}

struct Hit
{
	int x = 0;
	int y = 0;
	float scale = 1.0f;
};

// 在截图里找 file.png，返回它左上角在图里的位置和命中的缩放。
static bool find_file_icon(
	const fs::path& screenshot,
	const fs::path& template_image,
	float preferred_scale,
	int tolerance,
	Hit& hit)
{
	// 模板可能是在 100% 下截的，也可能已经是当前缩放下的像素：两种都试，
	// 再往 preferred_scale 上下各挪一档。每个缩放都是一次全图扫描，别试太多。
	std::vector<float> scales{preferred_scale, 1.0f, preferred_scale - 0.1f, preferred_scale + 0.1f};

	const std::string image = screenshot.string();
	const std::string templ = template_image.string();

	std::vector<float> tried;
	for (const float scale : scales)
	{
		if (scale < 0.05f)
		{
			continue;
		}
		const bool duplicated = std::any_of(
			tried.begin(), tried.end(), [scale](float value) { return std::fabs(value - scale) < 0.001f; });
		if (duplicated)
		{
			continue;
		}
		tried.push_back(scale);

		const int count =
			matchTemplatesSelectedScaleBegin(image.c_str(), templ.c_str(), tolerance, 1, scale);
		if (count < 0)
		{
			matchTemplateEnd();
			std::cerr << "错误: 模板匹配加载失败（" << count << "）: 截图 " << image
					  << " / 模板 " << templ << std::endl;
			return false;
		}
		if (count == 0)
		{
			matchTemplateEnd();
			continue;
		}

		const Point point = matchTemplateNext(0);
		matchTemplateEnd();
		hit.x = static_cast<int>(point.x);
		hit.y = static_cast<int>(point.y);
		hit.scale = scale;
		return true;
	}
	return false;
}

// ---- 参数 ---------------------------------------------------------------

struct Options
{
	std::vector<fs::path> inputs;
	int ready_x = readyToSendX;
	int ready_y = readyToSendY;
	int tolerance = DefaultTolerance;
	float scale = 0.0f; // >0 时覆盖 config.ini
	int wait_ms = 5000;
	bool dry_run = false;
	bool no_send = false;
};

static void print_usage()
{
	std::cout << "用法: UploadFiles.exe <目录或文件> [更多文件...]"
#ifdef _DEBUG	
		" [选项]"
#endif	
		"\n"
			  << "\n"
			  << "  目录会展开成它下面的所有文件（不递归）；文件按给的顺序上传。\n"
			  << "\n"

#ifdef _DEBUG

			  << "选项:\n"
			  << "  --ready X,Y      发送按钮的位置（默认 " << readyToSendX << "," << readyToSendY
			  << "）\n"
			  << "  --tolerance N    模板匹配容差（默认 " << DefaultTolerance << "）\n"
			  << "  --scale F        覆盖 config.ini 里的 scale\n"
			  << "  --wait MS        等上传对话框的毫秒数（默认 5000）\n"
			  << "  --dry-run        只截图 + 定位，不点击也不上传\n"
			  << "  --no-send        上传完不发送\n"
			  << "  -h, --help       显示帮助\n"
#endif
		;
}

static bool parse_pair(const std::wstring& text, int& first, int& second)
{
	const size_t comma = text.find(L',');
	if (comma == std::wstring::npos)
	{
		return false;
	}
	try
	{
		first = std::stoi(text.substr(0, comma));
		second = std::stoi(text.substr(comma + 1));
	}
	catch (const std::exception&)
	{
		return false;
	}
	return true;
}

// 返回 false 表示参数有问题（原因已经打印过）。
static bool parse_args(int argc, wchar_t** argv, Options& options, bool& show_help)
{
	show_help = false;
	for (int i = 1; i < argc; ++i)
	{
		const std::wstring arg = argv[i];

#ifdef _DEBUG



		auto value_after = [&](const char* name) -> const wchar_t*
		{
			if (i + 1 >= argc)
			{
				std::cerr << "错误: " << name << " 后面缺参数" << std::endl;
				return nullptr;
			}
			return argv[++i];
		};

		if (arg == L"-h" or arg == L"--help")
		{
			show_help = true;
			return true;
		}
		if (arg == L"--dry-run")
		{
			options.dry_run = true;
			continue;
		}
		if (arg == L"--no-send")
		{
			options.no_send = true;
			continue;
		}
		if (arg == L"--ready")
		{
			const wchar_t* value = value_after("--ready");
			if (value == nullptr)
			{
				return false;
			}
			if (not parse_pair(value, options.ready_x, options.ready_y))
			{
				std::cerr << "错误: --ready 要写成 X,Y" << std::endl;
				return false;
			}
			continue;
		}
		if (arg == L"--tolerance")
		{
			const wchar_t* value = value_after("--tolerance");
			if (value == nullptr)
			{
				return false;
			}
			options.tolerance = std::stoi(value);
			continue;
		}
		if (arg == L"--scale")
		{
			const wchar_t* value = value_after("--scale");
			if (value == nullptr)
			{
				return false;
			}
			options.scale = static_cast<float>(std::stod(value));
			continue;
		}
		if (arg == L"--wait")
		{
			const wchar_t* value = value_after("--wait");
			if (value == nullptr)
			{
				return false;
			}
			options.wait_ms = std::stoi(value);
			continue;
		}
#endif // _DEBUG
		options.inputs.emplace_back(arg);
	}
	return true;
}

// 目录展开成文件列表；文件原样保留。统一转成绝对路径（DLL 里按当前目录解析相对路径）。
static bool collect_files(const std::vector<fs::path>& inputs, std::vector<fs::path>& files)
{
	for (const fs::path& input : inputs)
	{
		std::error_code error;
		if (fs::is_directory(input, error))
		{
			std::vector<fs::path> found;
			for (const auto& entry : fs::directory_iterator(
					 input, fs::directory_options::skip_permission_denied, error))
			{
				if (entry.is_regular_file(error))
				{
					found.push_back(entry.path());
				}
			}
			std::sort(found.begin(), found.end());
			if (found.empty())
			{
				std::cerr << "警告: 目录里没有文件: " << input.string() << std::endl;
			}
			for (const auto& path : found)
			{
				files.push_back(fs::absolute(path));
			}
			continue;
		}

		if (fs::is_regular_file(input, error))
		{
			files.push_back(fs::absolute(input));
			continue;
		}

		std::cerr << "错误: 既不是文件也不是目录: " << input.string() << std::endl;
		return false;
	}
	return true;
}

// ---- main ---------------------------------------------------------------

int wmain(int argc, wchar_t** argv)
{
	SetConsoleOutputCP(CP_UTF8);

	Options options;
	bool show_help = false;
	if (not parse_args(argc, argv, options, show_help))
	{
		return 1;
	}
	if (show_help)
	{
		print_usage();
		return 0;
	}
	if (argc <= 1)
	{
		print_usage();
		return 1;
	}

	std::vector<fs::path> files;
	if (not collect_files(options.inputs, files))
	{
		return 1;
	}
	if (files.empty())
	{
		return fail("没有要上传的文件", 1);
	}

	// 1. DPI 感知要在截图之前：之后拿到的都是物理像素，点击坐标才对得上。
	DPIAwarenessPrologue();

	Settings settings = read_settings();
	if (options.scale > 0.0f)
	{
		settings.scale = options.scale;
	}
	log_line("scale: " + std::to_string(settings.scale) + "，窗口 " + std::to_string(settings.width) +
			 "x" + std::to_string(settings.height));

	const fs::path template_image = resolve_template();
	if (template_image.empty())
	{
		return fail(std::string("找不到模板 ") + TEMPLATE_FILE + "（要放在 exe 目录或当前目录）", 3);
	}


	int template_width = 0;
	int template_height = 0;
	if (not png_size(template_image, template_width, template_height))
	{
		return fail("模板不是有效的 PNG: " + template_image.string(), 3);
	}

	log_line("Files: " + std::to_string(files.size()) );
	//for (const auto& file : files)
	//{
	//	//log_line("  " + file.string());
	//}

	// 2. 截图（先删旧的，免得截图失败时误用上一次的图）。
	std::error_code ignored;
	fs::remove(SCREENSHOT_FILE, ignored);
	if (screenshot(
		static_cast<int>(UPLOAD_FILE_POSSIBLE_BBOX_ABSOLUTE_SIZE.left * settings.scale),
		static_cast<int>(UPLOAD_FILE_POSSIBLE_BBOX_ABSOLUTE_SIZE.top * settings.scale),
		static_cast<int>(UPLOAD_FILE_POSSIBLE_BBOX_ABSOLUTE_SIZE.right * settings.scale),
		static_cast<int>(UPLOAD_FILE_POSSIBLE_BBOX_ABSOLUTE_SIZE.bottom * settings.scale)) != 0)
	{
		return fail("截图失败（screenshot.png 没生成）", 2);
	}

	// 3. 定位 file.png。
	Hit hit;
	if (not find_file_icon(SCREENSHOT_FILE, template_image, settings.scale, options.tolerance, hit))
	{
		return fail("截图里没找到 file.png：确认 QQ 窗口在前面，并且那个\"文件\"图标能看到", 3);
	}
	const int click_x = hit.x + static_cast<int>(std::lround(template_width * hit.scale / 2.0f))+ static_cast<int>(UPLOAD_FILE_POSSIBLE_BBOX_ABSOLUTE_SIZE.left * settings.scale);
	const int click_y = hit.y + static_cast<int>(std::lround(template_height * hit.scale / 2.0f))+ static_cast<int>(UPLOAD_FILE_POSSIBLE_BBOX_ABSOLUTE_SIZE.top * settings.scale);
	if (options.dry_run)
	{
		log_line("--dry-run");
		return 0;
	}

	// 4. 点开上传对话框，等它出来之后再等 1 秒。
	if (not Lclick(static_cast<unsigned>(click_x), static_cast<unsigned>(click_y)))
	{
		return fail("点击 file.png 失败", 4);
	}
	if (not wait_for_dialog(true, options.wait_ms))
	{
		return fail("上传对话框没出来（标题里没有\"请选择\"）", 5);
	}
	sleep_ms(1000);

	// 5. 一次把所有文件填进去并点"打开"。
	std::vector<std::wstring> wide_paths;
	wide_paths.reserve(files.size());
	for (const auto& file : files)
	{
		wide_paths.push_back(file.wstring());
	}
	std::vector<const wchar_t*> raw_paths;
	raw_paths.reserve(wide_paths.size());
	for (const auto& path : wide_paths)
	{
		raw_paths.push_back(path.c_str());
	}

	log_line("上传 " + std::to_string(raw_paths.size()) + " 个文件 ...");
	uploadSelectedFiles(raw_paths.data(), raw_paths.size());
	if (not wait_for_dialog(false, options.wait_ms))
	{
		return fail("上传对话框没有关掉，文件可能没选上", 6);
	}
	sleep_ms(1000);

	// 6. 点发送。
	if (options.no_send)
	{
		log_line("--no-send");
		return 0;
	}
	log_line("点击发送 (" + std::to_string(options.ready_x) + "," + std::to_string(options.ready_y) + ")");
	if (not Lclick(static_cast<unsigned>(options.ready_x), static_cast<unsigned>(options.ready_y)))
	{
		return fail("点击发送失败", 4);
	}

	log_line("完成。");
	return 0;
}
