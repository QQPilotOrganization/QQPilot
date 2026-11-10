
// SauceDlg.cpp: 实现文件
//

#include "pch.h"
#include "framework.h"
#include "Sauce.h"
#include "SauceDlg.h"
#include "afxdialogex.h"

#include"seasoning.h"

#ifdef _DEBUG
#define new DEBUG_NEW
#endif




// CSauceDlg 对话框
CSauceDlg::CSauceDlg(CWnd* pParent /*=nullptr*/)
	: CDialogEx(IDD_SAUCE_DIALOG, pParent)
	, Value(_T(""))
{
	m_hIcon = AfxGetApp()->LoadIcon(IDR_MAINFRAME);
}

void CSauceDlg::DoDataExchange(CDataExchange* pDX)
{
	CDialogEx::DoDataExchange(pDX);
	DDX_Text(pDX, IDC_EDIT1, Value);
	DDX_Control(pDX, IDC_EDIT1, FInp);
	DDX_Control(pDX, IDC_CHECK1, AutoValue);
	DDX_Control(pDX, IDC_EDIT3, Sinput);
}

BEGIN_MESSAGE_MAP(CSauceDlg, CDialogEx)
	ON_WM_PAINT()
	ON_WM_QUERYDRAGICON()
	ON_BN_CLICKED(IDOK, &CSauceDlg::OnBnClickedOk)
	ON_EN_CHANGE(IDC_EDIT1, &CSauceDlg::OnEnChangeEdit1)
	ON_BN_CLICKED(IDC_BUTTON1, &CSauceDlg::OnBnClickedButton1)
END_MESSAGE_MAP()




static std::generator<std::wstring> allFiles(const wchar_t* fileNames, const wchar_t dli = L',')
{

	std::wstring f;
	for (size_t i = 0; i < wcslen(fileNames); i++)
	{
		if (fileNames[i] == dli)
		{
			co_yield f;
			f = L"";
			continue;

		}
		auto fa = fileNames[i];
		f += fa;
	}
	co_yield f;

}


// CSauceDlg 消息处理程序

BOOL CSauceDlg::OnInitDialog()
{
	CDialogEx::OnInitDialog();

	// 设置此对话框的图标。  当应用程序主窗口不是对话框时，框架将自动
	//  执行此操作
	SetIcon(m_hIcon, TRUE);			// 设置大图标
	SetIcon(m_hIcon, FALSE);		// 设置小图标

	// TODO: 在此添加额外的初始化代码

	return TRUE;  // 除非将焦点设置到控件，否则返回 TRUE
}

// 如果向对话框添加最小化按钮，则需要下面的代码
//  来绘制该图标。  对于使用文档/视图模型的 MFC 应用程序，
//  这将由框架自动完成。

void CSauceDlg::OnPaint()
{
	if (IsIconic())
	{
		CPaintDC dc(this); // 用于绘制的设备上下文

		SendMessage(WM_ICONERASEBKGND, reinterpret_cast<WPARAM>(dc.GetSafeHdc()), 0);

		// 使图标在工作区矩形中居中
		int cxIcon = GetSystemMetrics(SM_CXICON);
		int cyIcon = GetSystemMetrics(SM_CYICON);
		CRect rect;
		GetClientRect(&rect);
		int x = (rect.Width() - cxIcon + 1) / 2;
		int y = (rect.Height() - cyIcon + 1) / 2;

		// 绘制图标
		dc.DrawIcon(x, y, m_hIcon);
	}
	else
	{
		CDialogEx::OnPaint();
	}
}

//当用户拖动最小化窗口时系统调用此函数取得光标
//显示。
HCURSOR CSauceDlg::OnQueryDragIcon()
{
	return static_cast<HCURSOR>(m_hIcon);
}
std::unique_ptr<wchar_t[]> CharToWchar(const char* src, UINT codePage = CP_ACP)
{
	if (src == nullptr) return nullptr;

	// 1. 计算所需宽字符数（含结尾 '\0'）
	int len = MultiByteToWideChar(codePage, 0, src, -1, nullptr, 0);
	if (len <= 0) return nullptr;

	// 2. 分配：make_unique 内部 new[]，unique_ptr 析构时 delete[]
	auto wstr = std::make_unique<wchar_t[]>(len);

	// 3. 实际转换
	if (MultiByteToWideChar(codePage, 0, src, -1, wstr.get(), len) <= 0)
		return nullptr;

	return wstr;
}

// 便捷重载：接受 std::string
std::unique_ptr<wchar_t[]> CharToWchar(const std::string& s, UINT codePage = CP_ACP)
{
	return CharToWchar(s.c_str(), codePage);
}
static std::filesystem::path copy_file_with_suffix(const std::filesystem::path& src)
{
	std::filesystem::path dst = src.parent_path()
		/ (src.stem().wstring() + L"s" + src.extension().wstring());
	try
	{

	std::filesystem::copy_file(src, dst, std::filesystem::copy_options::overwrite_existing);
	}
	catch (const std::exception& e)
	{
		//auto w = e.what();
		auto err = CharToWchar(e.what());
		AfxMessageBox(err.get(), MB_OK | MB_ICONERROR);
	}
	return dst;
}
void CSauceDlg::OnBnClickedOk()
{
	CString SInput;
	Sinput.GetWindowTextW(SInput);
	if (AutoValue.GetCheck() == BST_CHECKED or SInput==L"")
	{

		time_t now = time(NULL);
		char buffer[26];
		errno_t err = ctime_s(buffer, sizeof(buffer), &now);
		std::string salt;
		std::random_device rd;
		std::mt19937 mt19937(rd());
		std::uniform_int_distribution<uint64_t> distrib((std::numeric_limits<uint64_t>::min)(), (std::numeric_limits<uint64_t>::max)());
		for (size_t i = 0; i < 100; i++)
		{
			salt += buffer;
			salt += std::to_string(now);
			salt += std::to_string(rand());
			salt += std::to_string(distrib(mt19937));
		}


		CString files;
		FInp.GetWindowTextW(files);
		files.GetBuffer();
		auto f = allFiles(files.GetBuffer());
		
		for (auto&& i : f)
		{
			
			if (i != L"")
			{
				auto suffix = copy_file_with_suffix(i);
		
				Seasoning::add(suffix.c_str(), salt.c_str(), strlen(salt.c_str()), 1);
				
			}
		}
 	}
	else
	{
		CString files;
		FInp.GetWindowTextW(files);
		files.GetBuffer();
		auto f = allFiles(files.GetBuffer());

		for (auto&& i : f)
		{

			if (i != L"")
			{
				auto suffix = copy_file_with_suffix(i);
				
				Seasoning::add(suffix.c_str(), SInput.GetString(), std::wcslen(SInput.GetString()), 1);

			}
		}

	}
	AfxMessageBox(L"完成。",MB_OK | MB_ICONINFORMATION);

}

void CSauceDlg::OnEnChangeEdit1()
{
	//FInp.GetCurrentGestureInfo
	// TODO:  如果该控件是 RICHEDIT 控件，它将不
	// 发送此通知，除非重写 CDialogEx::OnInitDialog()
	// 函数并调用 CRichEditCtrl().SetEventMask()，
	// 同时将 ENM_CHANGE 标志“或”运算到掩码中。

	// TODO:  在此添加控件通知处理程序代码
}

void CSauceDlg::OnBnClickedButton1()
{
	BOOL isOpen = TRUE;
	CString defaultDir = L"";
	CString fileName = L"";
	CString filter = L"文件 (*.*)|*.*||";
	CFileDialog openFileDlg(isOpen, defaultDir, fileName, OFN_ALLOWMULTISELECT, filter, NULL);
	INT_PTR result = openFileDlg.DoModal();
	//std::stringstream outputFiles;
	CString outputFiles;
	if (result == IDOK) {
		POSITION position = openFileDlg.GetStartPosition();
		while (position != NULL)
		{
			CString pthName = openFileDlg.GetNextPathName(position);
			outputFiles += pthName.GetString();
			outputFiles += L",";
		}

		FInp.SetWindowTextW(outputFiles);
	}
}
