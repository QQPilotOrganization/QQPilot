// UploadFiles.cpp : 把一批文件传到 QQ 聊天窗口，并点"发送"。

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
#include <concepts>
#include<utility>
#include <array>

#include "UploadFileTemplate.h"
namespace fs = std::filesystem;


static FARPROC GetProcAny(HMODULE h, const char* name)
{
	FARPROC p = GetProcAddress(h, name);
	if (p) return p;

	char buf[256];
	_snprintf_s(buf, _TRUNCATE, "_%s", name);
	return GetProcAddress(h, buf);   // x86 备用
}

template <class T>
static bool Bind(HMODULE h, const char* name, T& fn)
{
	fn = reinterpret_cast<T>(GetProcAny(h, name));
	return fn != nullptr;
}

// ---------------- ScreenCapture.dll ----------------
struct ScreenCaptureApi
{
	HMODULE h = nullptr;
	PFN_fullScreenshot fullScreenshot = nullptr;
	PFN_screenshot     screenshot = nullptr;

	bool Load(const wchar_t* dll = L"ScreenCapture.dll")
	{
		h = LoadLibraryW(dll);
		if (!h) return false;
		return Bind(h, "fullScreenshot", fullScreenshot)
			&& Bind(h, "screenshot", screenshot);
	}
	ScreenCaptureApi() = default;
	ScreenCaptureApi(const
		ScreenCaptureApi&) = delete;
	ScreenCaptureApi& operator=(const
		ScreenCaptureApi&) = delete; ~
		ScreenCaptureApi() {
		if (h) { FreeLibrary(h); h = nullptr; }
	}
};

// ---------------- Vision.dll ----------------
struct VisionApi
{
	HMODULE h = nullptr;
	PFN_matchTemplatesSelectedScaleBegin matchTemplatesSelectedScaleBegin = nullptr;
	PFN_matchTemplateNext                matchTemplateNext = nullptr;
	PFN_matchTemplateEnd                 matchTemplateEnd = nullptr;

	bool Load(const wchar_t* dll = L"Vision.dll")
	{
		h = LoadLibraryW(dll);
		if (!h) return false;
		return Bind(h, "matchTemplatesSelectedScaleBegin", matchTemplatesSelectedScaleBegin)
			&& Bind(h, "matchTemplateNext", matchTemplateNext)
			&& Bind(h, "matchTemplateEnd", matchTemplateEnd);
	}
	VisionApi() = default;
	VisionApi(const
		VisionApi&) = delete;
	VisionApi& operator=(const
		VisionApi&) = delete; ~
		VisionApi() {
		if (h) { FreeLibrary(h); h = nullptr; }
	}
};

// ---------------- InputEvent.dll ----------------
struct InputEventApi
{
	HMODULE h = nullptr;
	PFN_DPIAwarenessPrologue DPIAwarenessPrologue = nullptr;
	PFN_Lclick               Lclick = nullptr;
	PFN_press				 press = nullptr;
	PFN_getVkKey			 getVkKey = nullptr;
	bool Load(const wchar_t* dll = L"InputEvent.dll")
	{
		h = LoadLibraryW(dll);
		if (!h) return false;
		return Bind(h, "DPIAwarenessPrologue", DPIAwarenessPrologue)
			and Bind(h, "Lclick", Lclick) and Bind(h,"press", press) and Bind(h,"getVkKey", getVkKey);
	}
	InputEventApi() = default;
	InputEventApi(const
		InputEventApi&) = delete;
	InputEventApi& operator=(const
		InputEventApi&) = delete; ~
		InputEventApi() {
		if (h) { FreeLibrary(h); h = nullptr; }
	}
};

// ---------------- uploadFile.dll ----------------
struct UploadFileApi
{
	HMODULE h = nullptr;
	PFN_uploadSelectedFiles uploadSelectedFiles = nullptr;
	PFN_escape              escapeFn = nullptr;

	bool Load(const wchar_t* dll = L"uploadFile.dll")
	{
		h = LoadLibraryW(dll);
		if (!h) return false;

		bool ok = Bind(h, "uploadSelectedFiles", uploadSelectedFiles);


		escapeFn = reinterpret_cast<PFN_escape>(GetProcAny(h, "escape"));

		return ok && escapeFn != nullptr;
	}
	UploadFileApi() = default;
	UploadFileApi(const
		UploadFileApi&) = delete;
	UploadFileApi& operator=(const
		UploadFileApi&) = delete; ~
		UploadFileApi() {
		if (h) { FreeLibrary(h); h = nullptr; }
	}
};
typedef std::array<float, 4> Rectf;
static constexpr Rectf toRelativRect(const RECT a, int width, int height)
{
	const  float left = static_cast<float>(a.left) / static_cast<float>(width);
	const  float top = static_cast<float>(a.top) / static_cast<float>(height);
	const  float right = static_cast<float>(a.right) / static_cast<float>(width);
	const  float bottom = static_cast<float>(a.bottom) / static_cast<float>(height);

	return 	 { left,top,right,bottom };
}

typedef std::pair<float, float> Pointf;
static constexpr Pointf toRelativePoint(int x, int y, int width, int height)
{
	return std::make_pair(
		static_cast<float>(x) / static_cast<float>(width),
		static_cast<float>(y) / static_cast<float>(height));

}


static std::array<int, 4> toActualSize(const Rectf& relative, const std::pair<int, int>& size)
{
	
	return {
			static_cast<int>(std::roundf(relative.at(0) * size.first)),
			static_cast<int>(std::roundf(relative.at(1) * size.second)),
			static_cast<int>(std::roundf(relative.at(2) * size.first)),
			static_cast<int>(std::roundf(relative.at(3) * size.second)),

	};

}
static std::pair<int, int> toActualPoint(Pointf relative, const std::pair<int, int>& size)
{
	return std::make_pair<int, int>(
		std::roundf(relative.first * static_cast<float>(size.first)),
		std::roundf(relative.second * static_cast<float>(size.second))
	);
}
constexpr int DefaultSizeW = 1280;
constexpr int DefaultSizeH = 720;

constexpr int BasicWidth = 2248;
constexpr int BasicHeight = 1260;

constexpr RECT UPLOAD_FILE_POSSIBLE_BBOX_ABSOLUTE_SIZE = { 350,920,350+859,920+99 };
constexpr Rectf UPLOAD_FILE_POSSIBLE_BBOX_RELATIVE_SIZE = toRelativRect(UPLOAD_FILE_POSSIBLE_BBOX_ABSOLUTE_SIZE, BasicWidth, BasicHeight);

// 截图/模板的固定文件名：DLL 内部写死，改不了。
const char* const SCREENSHOT_FILE = "screenshot.png";
const char* const TEMPLATE_FILE = "file.png";

const wchar_t* const DIALOG_TITLE_KEYWORD = L"请选择";


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

	static const unsigned char signature[8] = { 0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A };
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
	Hit& hit, const VisionApi& vision)
{
	std::vector<float> scales{ preferred_scale, preferred_scale - 0.1f, preferred_scale + 0.1f };

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
			vision.matchTemplatesSelectedScaleBegin(image.c_str(), templ.c_str(), tolerance, 1, scale);
		if (count < 0)
		{
			vision.matchTemplateEnd();
			std::cerr << "错误: 模板匹配加载失败（" << count << "）: 截图 " << image
				<< " / 模板 " << templ << std::endl;
			return false;
		}
		if (count == 0)
		{
			vision.matchTemplateEnd();
			continue;
		}

		const Point point = vision.matchTemplateNext(0);
		vision.matchTemplateEnd();
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
	int tolerance = DefaultTolerance;
	float scale = 0.0f; // >0 时覆盖 config.ini
	int wait_ms = 5000;
	bool dry_run = false;
	bool no_send = false;
	bool screenshotOnly = false;
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
		if (arg == L"-ss")
		{
			options.screenshotOnly = true;
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
				std::cerr << "目录里没有文件: " << input.string() << std::endl;
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

		std::cerr << " 既不是文件也不是目录: " << input.string() << std::endl;
		return false;
	}
	return true;
}

// ---- main ---------------------------------------------------------------

static int doScreenshot(const ScreenCaptureApi& screenCapture, const std::array<int, 4>& bbox);

int wmain(int argc, wchar_t** argv)
{
	SetConsoleOutputCP(CP_UTF8);

	ScreenCaptureApi screenCapture;
	VisionApi        vision;
	InputEventApi    inputEvent;
	UploadFileApi    uploadFile;

	bool fullyLoaded = true;

	if (not screenCapture.Load() )	 
	{
		std::cout << "未找到ScreenCapture.dll\n";
		fullyLoaded = false;
	}

	if (not vision.Load())
	{
		std::cout << "未找到vision.dll\n";
		fullyLoaded = false;
	}
	if (not inputEvent.Load())
	{
		std::cout << "未找到inputEvent.dll\n";
		fullyLoaded = false;
	}

	if (not uploadFile.Load())
	{
		std::cout << "未找到uploadFile.dll\n";
		fullyLoaded = false;
	}
	if (not fullyLoaded)
	{
		return 1;
	}


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


	bin2cpp::getUploadFileTemplatePngFile().save("file.png");

	Settings settings = read_settings();


	if (options.scale > 0.0f)
	{
		settings.scale = options.scale;
	}

	std::pair<int, int> actualWindowSize = std::make_pair<int, int>(settings.width * settings.scale, settings.height * settings.scale);
	std::array<int, 4> uploadFilePossibleBBoxActualSize = toActualSize(UPLOAD_FILE_POSSIBLE_BBOX_RELATIVE_SIZE, actualWindowSize);

	if (options.screenshotOnly)
	{
		doScreenshot(screenCapture,uploadFilePossibleBBoxActualSize);
		log_line("截图成功。");
		return 0;
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

	inputEvent.DPIAwarenessPrologue();




	log_line("scale: " + std::to_string(settings.scale) + "，窗口 " + std::to_string(actualWindowSize.first) +
		"x" + std::to_string(actualWindowSize.second));

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

	log_line("Files: " + std::to_string(files.size()));
	// 2. 截图（先删旧的，免得截图失败时误用上一次的图）。
	std::error_code ignored;
	fs::remove(SCREENSHOT_FILE, ignored);
	if (doScreenshot(screenCapture, uploadFilePossibleBBoxActualSize) != 0)
	{
		return fail("截图失败（screenshot.png 没生成）", 2);
	}

	// 3. 定位 file.png。
	Hit hit;
	if (not find_file_icon(SCREENSHOT_FILE, template_image, settings.scale, options.tolerance, hit, vision))
	{
		return fail("截图里没找到 file.png：确认 QQ 窗口在前面，并且那个\"文件\"图标能看到", 3);
	}
	const int click_x = hit.x + static_cast<int>(std::lround(template_width * hit.scale / 2.0f)) + uploadFilePossibleBBoxActualSize.at(0);
	const int click_y = hit.y + static_cast<int>(std::lround(template_height * hit.scale / 2.0f)) + uploadFilePossibleBBoxActualSize.at(1);
	if (options.dry_run)
	{
		log_line("--dry-run");
		return 0;
	}

	// 4. 点开上传对话框，等它出来之后再等 1 秒。
	if (not inputEvent.Lclick(static_cast<unsigned>(click_x), static_cast<unsigned>(click_y)))
	{
		return fail("点击 file.png 失败", 4);
	}
	if (not wait_for_dialog(true, options.wait_ms))
	{
		uploadFile.escapeFn();
	}
	if (not wait_for_dialog(true, options.wait_ms))
	{
		return fail("上传对话框不可见", 5);
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
	uploadFile.uploadSelectedFiles(raw_paths.data(), raw_paths.size());
	if (not wait_for_dialog(false, options.wait_ms))
	{
		uploadFile.escapeFn();
	}
	if (not wait_for_dialog(false, options.wait_ms))
	{
		return fail("上传对话框未关闭", 6);
	}
	sleep_ms(1000);

	// 6. 点发送。
	if (options.no_send)
	{
		log_line("--no-send");
		return 0;
	}
	sleep_ms(1000);
	inputEvent.press(inputEvent.getVkKey("enter"));
	//inputEvent
	log_line("完成。");
	uploadFile.escapeFn();

	return 0;
}

static int doScreenshot(const ScreenCaptureApi& screenCapture,const std::array<int, 4>& bbox)
{
	return screenCapture.screenshot(
		static_cast<int>(bbox.at(0)),
		static_cast<int>(bbox.at(1)),
		static_cast<int>(bbox.at(2)),
		static_cast<int>(bbox.at(3)));
}
