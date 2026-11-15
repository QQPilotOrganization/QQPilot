
// MFCLauncherDlg.h: 头文件
//

#pragma once

#include <memory>
#include <string>

#include "localization.h"

// CMFCLauncherDlg 对话框
class CMFCLauncherDlg : public CDialogEx
{
// 构造
public:
	CMFCLauncherDlg(CWnd* pParent = nullptr);	// 标准构造函数

// 对话框数据
#ifdef AFX_DESIGN_TIME
	enum { IDD = IDD_MFCLAUNCHER_DIALOG };
#endif

	protected:
	virtual void DoDataExchange(CDataExchange* pDX);	// DDX/DDV 支持


// 实现
protected:
	HICON m_hIcon;

	// 翻译：每个对话框自己一份实例（在 OnInitDialog 里创建，那时 CWD 已经切到 data）
	std::unique_ptr<Clocalization> m_i18n;

	// 取一条文案；没加载成功时返回 X<key>
	std::wstring T(const char* key) const;
	// 给某个控件设置文案（控件不存在就跳过）
	void SetText(int id, const char* key);
	// 启动子进程（文件名相对于 data 目录）
	bool run(const wchar_t* fileName);

	// 生成的消息映射函数
	virtual BOOL OnInitDialog();
	afx_msg void OnPaint();
	afx_msg HCURSOR OnQueryDragIcon();
	DECLARE_MESSAGE_MAP()
public:
	afx_msg void OnBnClickedOk();
	afx_msg void OnBnClickedButton1();
	afx_msg void OnBnClickedButton4();
	afx_msg void OnBnClickedButton3();
	afx_msg void OnBnClickedButton5();
	afx_msg void OnBnClickedButton2();
	afx_msg void OnBnClickedOk2();
	afx_msg void OnBnClickedButton6();
};
