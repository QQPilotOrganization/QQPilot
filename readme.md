```plain text
   ____     ____    _____    _   _           _   
  / __ \   / __ \  |  __ \  (_) | |         | |  
 | |  | | | |  | | | |__) |  _  | |   ___   | |_ 
 | |  | | | |  | | |  ___/  | | | |  / _ \  | __|
 | |__| | | |__| | | |      | | | | | (_) | | |_ 
  \___\_\  \___\_\ |_|      |_| |_|  \___/   \__|
                                        
```
# QQPilot - 基于窗口自动化的 QQ 自动回复机器人


[Linux版本](https://github.com/QQPilotOrganization/QQPilotLinux)
[Android](https://github.com/QQPilotOrganization/QQPilotPocketEdition)



<!-- [![示例截图](./QQPilot.jpeg)](./QQPilot.jpeg) -->
<div align="center">

<img alt="示例截图" src="./assets/qqpilot.png" width="300" >
</div>



> 使用纯视觉 + 窗口自动化实现 QQ 消息自动回复，**零 API 依赖、零注入、低封号风险**。  

##  项目简介

QQPilot 是一个全自动的 QQ 聊天机器人，通过以下流程实现智能回复：

> **复制聊天内容 → 解析消息（含图片/表情包）→ 调用 LLM 生成回复 → 模拟输入并发送**

全程 **不调用 QQ 内部接口、不 Hook 进程、不注入 DLL**，极大降低账号封禁风险。

## 1.5.17

对于强制使用Ollama API，填写类似https://example.com 即可，会自动定向到 https://example.com/api/chat。
否则填写https://example.com/v1 定向到 https://example.com/v1/chat/completions/

---

## 安装指南

### 步骤 1：下载项目
确保已经安装[.NET10](https://dotnet.microsoft.com/en-us/download/dotnet/thank-you/sdk-10.0.203-windows-x64-installer)

> * **1.5.19+无需安装。**

前往 [Releases 页面](https://github.com/Na2Cr2O7/QQPilot/releases) 下载最新压缩包并解压。

解压后大小<6M.

安装[QQ](https://im.qq.com/index/#/)


### 步骤 2：配置用于回复的模型
确保你已经拥有API提供商的API-Key或者配置了本地模型。

QQPilot只支持Ollama API 或者Chat Completion API。


安装 [Ollama](https://ollama.com/) 并拉取模型：

```bash
# 推荐主力模型（9B，性能与效果平衡）
ollama pull qwen3.5:9b

# 低配设备可选（1B，轻量快速）
ollama pull minicpm-v4.6:1b
```

* 可以在虚拟机上使用宿主机的Ollama，详情见[教程](useollamainVM/useOllamainVM.md)


### 步骤 3：初始化设置
运行 `菜单.exe` 

![menu2](assets/menu2.png)

并打开 `设置` 配置。
![alt text](assets/option2.png)

#### 各配置项用法


| 设置项             | 解释                     |
|--------------------|---------------------------|
| 用户名         | 判断是否是自身的消息。填写机器人账号的昵称。建议在群聊中，不要修改昵称，否则会导致LLM无法正确识别到@命令        |
| 窗口宽度和高度     | 程序启动后会移动QQ到最左上角并调至该大小            |
|Token用量    | 基于API的参数计算            |
|解析图片    | 只会将选定的图片数量传给API            |
|模型名称    | 填写使用的模型          |
|视觉模型    | 选定的模型是否是视觉模型，如果不是，则不会传任何图片给API          |
|API Key|填写LLM 提供商的API Key，如果是Ollama，可以填写随机值|
|服务器|支持直接使用Ollama(http://localhost:11434/api/chat),内置模型（Jaccard）和填写URL。填写类似https://example.com/v1 定向到 https://example.com/v1/chat/completions/，若开启 **强制使用Ollama API** ，填写类似https://example.com 即可，会自动定向到 https://example.com/api/chat|
|框选消息时长|选择消息的长度随时长的增加而增加|
|请求的额外参数|API请求的额外参数，`{"think":false}`可以让Ollama API 的模型不思考 |
|包含图片|发送完结果后上传`.\Images`下面的图片，API返回的图片必定发送 |
|自动点击登录|启动后自动寻找登录按钮并点击（建议使用QQ的自动登录） |
|持续将窗口置于最前|将QQ窗口置于最前防止遮挡|
|发送图片概率|概率选择`.\Images`下面的图片并发送|
|远程服务器超时|在时间到后关闭连接，对于性能较差的计算机，使用本地模型时建议保持`300`|
|tab按下次数|模板匹配失败后才需要用到，如果点到了删除按钮，请降低|
|提示文本|System Prompt|

### 步骤4、启动

 - （可选）将自定义表情包放入 `.\Images` 文件夹 

 - **确保 QQ 主窗口始终可见（不要最小化或遮挡）**  

 - ****运行时请勿移动鼠标！！！！！****
 - ****运行时请勿移动鼠标！！！！！****
 - ****运行时请勿移动鼠标！！！！！****

#### 启动QQ并登录机器人账号并配置

| 设置项             |                    |
|--------------------|---------------------------|
| **发送消息**          | **Ctrl+Enter**             |
| 联系人面板宽度     | 拖动至**最窄**             |
|主题|**浅色主题**|
> 🔍 QQPilot 通过 UI 坐标识别消息，任何界面变动（如缩放、深色主题）都可能导致识别失败。


在`菜单.exe`中选择`启动QQPilot`。
 - 程序将自动监控未读消息并智能回复

正常运行时应该如下
![alt text](./assets/running.png)

 ******运行期间请勿更改 DPI/分辨率！******
 - ****运行时请勿移动鼠标！！！！！****
 - ****运行时请勿移动鼠标！！！！！****
 - ****运行时请勿移动鼠标！！！！！****


---

## 核心优势

- **安全**：纯视觉操作，零注入、零 Hook，几乎无封号风险  
- **隐私**：支持完全本地运行，数据不出设备  
- **灵活**：可对接任意Open AI API本地大模型（如 Ollama）或远程 HTTP API  

<div align="center">

<img alt="示例截图" src="./assets/banner1.png" >
</div>
---


## 配置要求

> 
>  ⚠️该程序不支持无头模式（至少外接一台显示器）
> 

### 最低要求

### x86_64

#### Windows
 - Windows 8.1 或更高版本 64位

 - 单核处理器，主频1GHz以上

 - 1GB RAM

 - 200M 可用空间

 - 1920x1080 显示器

> 对于Windows7 可以尝试安装 [VkKex](https://github.com/YuZhouRen86/VxKex-NEXT)



### ARM64

 - Windows 10 ARM64 及以上

 - 2GB RAM

 - 200M 可用空间

 - 1920x1080 显示器


### 推荐配置

 - Windows 11 x64

 - 4核心，2GHz以上

 - 4GB RAM

 - 12GB 可用空间(用于Ollama)

 - 支持CUDA的GPU

- 1920x1080 显示器
 

---

## ⚠️ 使用限制

- 需图形界面，不支持服务器/远程桌面无头模式/窗口管理器  
- 对 **屏幕分辨率** 和 **DPI 缩放比例** 敏感（推荐设置为 **100% 或 125%**）  
- QQ 主窗口必须 **可见且未最小化**（不可被其他窗口遮挡）

---

## 🔧 工作原理

1. **窗口置顶**  
   通过 `FocusQQWindow.dll`强制将 QQ 主窗口置顶，确保截图一致性。

2. **DPI 自适应**  
   运行 `ScaleToINI.exe` 自动检测系统缩放比例，并写入 `config.ini` 用于坐标校准。

3. **未读消息检测**  
   在联系人列表区域扫描“小红点”，定位有新消息的会话。

4. **自动交互**  
   模拟鼠标点击红点位置，打开对应聊天窗口。

5. **内容识别**  
   - 使用框选功能提取聊天内容（含文本与图片）
   - 示例格式：
     ```markdown
     Username: 11-01 08:12:19
     <img src="file://C:/Image.png" />
     内容内容内容...

     aaaaaaaaaaa: 11-25 08:10:36
     普通文字，没有图片

     bbbbbbbbbbb: 11-25 08:11:00
     <img src="file:///C:/Users/Admin/Pictures/test%20image.png" />
     还有这张图！
     ```
   - 支持提取普通文本、时间戳、用户昵称及本地图片路径
   - 可识别并提取表情包（需启用视觉模型）

6. **智能回复生成**  
   将解析后的内容传入本地大模型（如 Ollama）或自定义 Chat Completion API，生成自然语言回复。

7. **自动发送**  
   - 将回复粘贴至 QQ 输入框  
   - （可选）随机插入 `.\Images` 中的表情包  
   - 模拟回车键发送消息

8. **会话清理**  
   发送完成后自动关闭当前聊天窗口，返回主界面继续监听。


---

## 🛠️ 编译说明（开发者）

使用 **Visual Studio 2026** 打开并编译解决方案：
- `VisionQQ_C.slnx`

使用你的rustIDE打开并编译:

- `qqpilot5`

- `option3`

---

## 🛡️ 免责声明

本软件 **仅限技术学习与研究用途**，严禁用于：
- 自动骚扰、刷屏、诈骗等恶意行为  
- 违反《QQ 软件许可协议》的操作  
- 任何违法违规场景

使用者须自行承担因使用本软件引发的一切法律责任，作者概不负责。

本项目采用 [MIT License](LICENSE)










