
// ImageImportDlg.h: 头文件
//

#pragma once

#include <memory>
#include <string>
#include <initializer_list>

#include "localization.h"

// CImageImportDlg 对话框
class CImageImportDlg : public CDialogEx
{
// 构造
public:
	CImageImportDlg(CWnd* pParent = nullptr);	// 标准构造函数

// 对话框数据
#ifdef AFX_DESIGN_TIME
	enum { IDD = IDD_IMAGEIMPORT_DIALOG };
#endif

	protected:
	virtual void DoDataExchange(CDataExchange* pDX);	// DDX/DDV 支持


// 实现
protected:
	HICON m_hIcon;

	// 翻译：每个对话框自己一份实例（在 OnInitDialog 里创建，那时 CWD 已经是对的）
	std::unique_ptr<Clocalization> m_i18n;

	// 取一条文案；没加载成功时返回 X<key>
	std::wstring T(const char* key) const;
	// 带 `{0}` `{1}` 占位符的文案
	std::wstring Format(const char* key, std::initializer_list<std::wstring> args) const;
	// 给某个控件设置文案（控件不存在就跳过）
	void SetText(int id, const char* key);

	// 生成的消息映射函数
	virtual BOOL OnInitDialog();
	afx_msg void OnPaint();
	afx_msg HCURSOR OnQueryDragIcon();
	DECLARE_MESSAGE_MAP()
public:
	afx_msg void OnBnClickedOk();
	afx_msg void OnEnChangeFolders();
    afx_msg void addLog(CString& Log);
	// a
	// a
	// a
	// a
	// a
	CEdit Value;
	CString Text;
	CString m_folder;
	afx_msg void OnBnClickedButton1();
	afx_msg void ThcopyFile();
	std::thread t;
	afx_msg void OnBnClickedButton2();
	CButton Log;
	CStatic Log2;
	afx_msg void OnStnClickedSe();
	afx_msg void OnBnClickedButton4();
	afx_msg void OnBnClickedButton3();
};
