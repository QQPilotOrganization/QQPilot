
// ImageImportDlg.cpp: 实现文件
//

#include "pch.h"
#include "framework.h"
#include "ImageImport.h"
#include "ImageImportDlg.h"
#include "afxdialogex.h"
#include "findTencentFiles.h"
#include "ImageSignature.h"
#include<string>
#include <afxdlgs.h>
#include <optional>
#include<memory>

#ifdef _DEBUG
#define new DEBUG_NEW
#endif
#include <generator>       //C++23
#include <filesystem>
#include <ranges>
#include <cctype>
constexpr auto IMAGE_DEST_PATH = L".\\Images\\";
static std::generator<std::filesystem::path> recursiveFiles(std::filesystem::path path)
{
	try
	{
		for (auto& p : std::filesystem::recursive_directory_iterator(path))
		{

			if (p.is_regular_file())
			{
				co_yield p.path();
			}
		}

	}
	catch (std::filesystem::filesystem_error& e)
	{

	}

}
static bool hasExtension(std::filesystem::path path, std::wstring_view ext)
{
	if (ext.empty())
	{
		return false;
	}
	std::wstring want(ext);
	if (ext[0] != L'.')
	{
		want = L".";
		want += ext;
	}
	auto actualExtension = path.extension().wstring();
	std::transform(actualExtension.begin(), actualExtension.end(), actualExtension.begin(), ::towlower);
	std::transform(want.begin(), want.end(), want.begin(), ::towlower);
	return actualExtension == want;
}

// CImageImportDlg 对话框



CImageImportDlg::CImageImportDlg(CWnd* pParent /*=nullptr*/)
	: CDialogEx(IDD_IMAGEIMPORT_DIALOG, pParent)
	, Text(_T(""))
{
	m_hIcon = AfxGetApp()->LoadIcon(IDR_MAINFRAME);
}

void CImageImportDlg::DoDataExchange(CDataExchange* pDX)
{
	CDialogEx::DoDataExchange(pDX);
	DDX_Control(pDX, INPUT_FOLDERS, Value);
	DDX_Text(pDX, INPUT_FOLDERS, Text);
	//DDX_Control(pDX, IDC_EDIT2, Log);
	//DDX_Control(pDX, IDC_RICHEDIT21, Log2);
	DDX_Control(pDX, IDC_BUTTON1, Log);
	DDX_Control(pDX, IDC_Se, Log2);
}

BEGIN_MESSAGE_MAP(CImageImportDlg, CDialogEx)
	ON_WM_PAINT()
	ON_WM_QUERYDRAGICON()
	ON_BN_CLICKED(IDOK, &CImageImportDlg::OnBnClickedOk)
	ON_EN_CHANGE(INPUT_FOLDERS, &CImageImportDlg::OnEnChangeFolders)
	ON_BN_CLICKED(IDC_BUTTON1, &CImageImportDlg::OnBnClickedButton1)
	ON_BN_CLICKED(IDC_BUTTON2, &CImageImportDlg::OnBnClickedButton2)
	ON_STN_CLICKED(IDC_Se, &CImageImportDlg::OnStnClickedSe)
	//ON_BN_CLICKED(IDC_BUTTON4, &CImageImportDlg::OnBnClickedButton4)
	//ON_BN_CLICKED(IDC_BUTTON3, &CImageImportDlg::OnBnClickedButton3)
END_MESSAGE_MAP()


// CImageImportDlg 消息处理程序

BOOL CImageImportDlg::OnInitDialog()
{
	CDialogEx::OnInitDialog();

	// 设置此对话框的图标。  当应用程序主窗口不是对话框时，框架将自动
	//  执行此操作
	SetIcon(m_hIcon, TRUE);			// 设置大图标
	SetIcon(m_hIcon, FALSE);		// 设置小图标

	// TODO: 在此添加额外的初始化代码
	std::optional<std::wstring> tencentFiles = findTencentFiles();
	std::wstring ntDataPath = L"";
	if (tencentFiles.has_value())
	{
		std::vector<std::filesystem::path> folders;
		listFolders(tencentFiles.value(), folders);
		for (auto& folder : folders)
		{
			if (folderValid(folder))
			{

				m_folder = (folders[0] / "nt_qq\\nt_data\\Emoji\\emoji-recv").c_str();
				ntDataPath = (folders[0] / "nt_qq\\nt_data\\Emoji\\emoji-recv");
				goto done;
			}

		}
		Log2.SetWindowTextW(L"完毕。");
		return TRUE;
	done:
		std::wstring str = L"找到nt_data文件夹：" + folders[0].wstring() + L"\n";
		CString str2 = str.c_str();
		addLog(str2);
		Value.SetWindowTextW(ntDataPath.c_str());
	}



	return TRUE;  // 除非将焦点设置到控件，否则返回 TRUE
}

// 如果向对话框添加最小化按钮，则需要下面的代码
//  来绘制该图标。  对于使用文档/视图模型的 MFC 应用程序，
//  这将由框架自动完成。

void CImageImportDlg::OnPaint()
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
HCURSOR CImageImportDlg::OnQueryDragIcon()
{
	return static_cast<HCURSOR>(m_hIcon);
}


void CImageImportDlg::OnBnClickedOk()
{
	// TODO: 在此添加控件通知处理程序代码
	CDialogEx::OnOK();
}

void CImageImportDlg::OnEnChangeFolders()
{
	// TODO:  如果该控件是 RICHEDIT 控件，它将不
	// 发送此通知，除非重写 CDialogEx::OnInitDialog()
	// 函数并调用 CRichEditCtrl().SetEventMask()，
	// 同时将 ENM_CHANGE 标志“或”运算到掩码中。

	// TODO:  在此添加控件通知处理程序代码
	Value.GetWindowTextW(m_folder);
}

void CImageImportDlg::addLog(CString& text)
{
	//CString LegacyLog;
	//Log2.GetWindowTextW(LegacyLog);
	Log2.SetWindowTextW(text);
}

void CImageImportDlg::OnBnClickedButton1()
{
	CFolderPickerDialog folderDialog;

	//folderDlg.SetWindowTextW(L"请选择一个文件夹");
	folderDialog.m_ofn.lpstrTitle = L"请选择";

	
	auto txFile = findTencentFiles();
	folderDialog.m_ofn.lpstrInitialDir = txFile.has_value() ? txFile.value().c_str() : L"";

	if (folderDialog.DoModal() == IDOK)
	{
		CString strFolderPath = folderDialog.GetPathName();
		// 显示或使用路径
		Value.SetWindowTextW(strFolderPath);
	}
}
void CImageImportDlg::ThcopyFile()
{
	auto result = recursiveFiles(m_folder.GetBuffer()) |
		std::views::filter([this](const std::filesystem::path& p)
			{
				if ((hasExtension(p, L".png") or hasExtension(p, L".jpg") or hasExtension(p, L".gif")))
				{
					CString name = std::wstring(L"正在处理：" + p.filename().wstring() + p.extension().wstring()).c_str();
					this->addLog(name);
					return true;
				}
				return false;
			}
		);
	int imageCount = 0;
	int successCount = 0;
	int failCount = 0;
	for (const auto& file : result)
	{
		wchar_t path[MAX_PATH];
		wcscpy_s(path, IMAGE_DEST_PATH);
		path[std::wcslen(IMAGE_DEST_PATH)] = '\0';
		if (not CreateDirectoryW(path, NULL))
		{
			if (GetLastError() == ERROR_PATH_NOT_FOUND)
			{
				MessageBox(L"一个或多个中间目录不存在", L"错误", MB_OK | MB_ICONERROR);
			}
		}
		//std::wstring realExt=;
		auto realExt = getRealExtension(file.c_str());
		std::wstring destFileName = file.stem().wstring();
		if (realExt.has_value())
		{
			destFileName += L"." + realExt.value();
		}
		else
		{
			destFileName += file.extension().wstring();
		}
		wcscat_s(path, destFileName.c_str());
		path[MAX_PATH - 1] = '\0';
		bool success = CopyFile(file.c_str(), path, FALSE);
		if (success)
		{

			CString name2 = std::wstring(L"成功复制："+file.filename().wstring()+L" -> " + destFileName).c_str();
			this->addLog(name2);
			++imageCount;
			++successCount;
		}
		else
		{
			DWORD err = GetLastError();
			CString name2 = (std::wstring(L"复制失败：" + std::to_wstring(err))).c_str();

			this->addLog(name2);
			++failCount;

		}


	}

	Log2.SetWindowText(L"复制完成！");
	std::wstring log = L"成功复制：" + std::to_wstring(successCount) + L"张，失败：" + std::to_wstring(failCount) + L"张，总共：" + std::to_wstring(imageCount) + L"张";
	MessageBox(log.c_str(), L"复制完成！", MB_OK | MB_ICONEXCLAMATION);



}
void CImageImportDlg::OnBnClickedButton2()
{
	// TODO: 在此添加控件通知处理程序代码
	bool success = CreateDirectory(TEXT(".\\Images"), NULL);
	if (not success)
	{
		DWORD error = GetLastError();
		if (error != ERROR_ALREADY_EXISTS)
		{
			AfxMessageBox(L"无法创建文件夹");
			return;
		}
	}

	if (not std::filesystem::exists(m_folder.GetBuffer()))
	{
		AfxMessageBox(L"文件夹不存在");
		return;
	}
	//this->t = std::thread(&CImageImportDlg::ThcopyFile,this);
	ThcopyFile();

}



void CImageImportDlg::OnStnClickedSe()
{
	// TODO: 在此添加控件通知处理程序代码
}

void CImageImportDlg::OnBnClickedButton4()
{
	wchar_t path[MAX_PATH + 10];
	wcscpy_s(path, L"explorer ");
	wcscat_s(path, m_folder.GetBuffer());


	//system(path);
}

void CImageImportDlg::OnBnClickedButton3()
{}

// TODO: 在此添加控件通知处理程序代码