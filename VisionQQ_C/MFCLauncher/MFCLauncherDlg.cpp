
// MFCLauncherDlg.cpp: 实现文件
//

#include "pch.h"
#include "framework.h"
#include "MFCLauncher.h"
#include "MFCLauncherDlg.h"
#include "afxdialogex.h"
bool run(const wchar_t* fn);
#ifdef _DEBUG
#define new DEBUG_NEW
#endif


// CMFCLauncherDlg 对话框



CMFCLauncherDlg::CMFCLauncherDlg(CWnd* pParent /*=nullptr*/)
	: CDialogEx(IDD_MFCLAUNCHER_DIALOG, pParent)
{
	m_hIcon = AfxGetApp()->LoadIcon(IDR_MAINFRAME);
}

void CMFCLauncherDlg::DoDataExchange(CDataExchange* pDX)
{
	CDialogEx::DoDataExchange(pDX);
}

BEGIN_MESSAGE_MAP(CMFCLauncherDlg, CDialogEx)
	ON_WM_PAINT()
	ON_WM_QUERYDRAGICON()
	ON_BN_CLICKED(IDOK, &CMFCLauncherDlg::OnBnClickedOk)
	ON_BN_CLICKED(IDC_BUTTON1, &CMFCLauncherDlg::OnBnClickedButton1)
	ON_BN_CLICKED(IDC_BUTTON4, &CMFCLauncherDlg::OnBnClickedButton4)
	ON_BN_CLICKED(IDC_BUTTON3, &CMFCLauncherDlg::OnBnClickedButton3)
	ON_BN_CLICKED(IDC_BUTTON5, &CMFCLauncherDlg::OnBnClickedButton5)
	ON_BN_CLICKED(IDC_BUTTON2, &CMFCLauncherDlg::OnBnClickedButton2)
	//ON_BN_CLICKED(IDOK2, &CMFCLauncherDlg::OnBnClickedOk2)
	ON_BN_CLICKED(IDC_BUTTON6, &CMFCLauncherDlg::OnBnClickedButton6)
END_MESSAGE_MAP()


// CMFCLauncherDlg 消息处理程序

BOOL CMFCLauncherDlg::OnInitDialog()
{
	CDialogEx::OnInitDialog();

	// 设置此对话框的图标。  当应用程序主窗口不是对话框时，框架将自动
	//  执行此操作
	SetIcon(m_hIcon, TRUE);			// 设置大图标
	SetIcon(m_hIcon, FALSE);		// 设置小图标

	// TODO: 在此添加额外的初始化代码
	SetCurrentDirectory(TEXT(".\\data\\"));

	return TRUE;  // 除非将焦点设置到控件，否则返回 TRUE
}

// 如果向对话框添加最小化按钮，则需要下面的代码
//  来绘制该图标。  对于使用文档/视图模型的 MFC 应用程序，
//  这将由框架自动完成。

void CMFCLauncherDlg::OnPaint()
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
HCURSOR CMFCLauncherDlg::OnQueryDragIcon()
{
	return static_cast<HCURSOR>(m_hIcon);
}


void CMFCLauncherDlg::OnBnClickedOk()
{
	// TODO: 在此添加控件通知处理程序代码
	CDialogEx::OnOK();
}

void CMFCLauncherDlg::OnBnClickedButton1()
{
	//system("start run.exe");
	run(L"QQPilot5.exe");
}

void CMFCLauncherDlg::OnBnClickedButton4()
{
	//run(L"Download.exe");

}
bool run(const wchar_t* fn)
{
	STARTUPINFOW si;
	PROCESS_INFORMATION pi;
	ZeroMemory(&si, sizeof(si));
	si.cb = sizeof(si);
	ZeroMemory(&pi, sizeof(pi));
	BOOL result= CreateProcessW(fn, NULL, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi);
    if (!result)
    {
        MessageBox(NULL,L"启动失败",L"",MB_OK | MB_ICONERROR);
    }
    return result;
}

void CMFCLauncherDlg::OnBnClickedButton3()
{
    run(L"Option3.exe");
	
}

void CMFCLauncherDlg::OnBnClickedButton5()
{
	run(L"ImageImport.exe");

}

void CMFCLauncherDlg::OnBnClickedButton2()
{
	//run(L"扩展管理器.exe");
	// TODO: 在此添加控件通知处理程序代码
}
													
void CMFCLauncherDlg::OnBnClickedOk2()
{
	// TODO: 在此添加控件通知处理程序代码
	//run(L"升级助手.exe");
								
}

void CMFCLauncherDlg::OnBnClickedButton6()
{
	//run(L"MFCUpdate.exe");
	// TODO: 在此添加控件通知处理程序代码
}
