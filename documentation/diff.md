## 不同平台的版本差异

| 差异           | QQPilot                     |FishCakeQQ|QQPilotPocketEdition|
|--------------------|---------------------------|-|-|
|编程语言|C/C++/rust|Python/JavaScript|JavaScript|
|可用平台|Windows|Linux+x11|Android|
|体积|**<6M**|<800M|<60M|
|复制聊天内容 → 解析消息（含图片/表情包）→ 调用 LLM 生成回复 → 模拟输入并发送|✅|✅|✅|
|上传图片|✅||
|扩展功能||✅停止维护|
|图形设置|✅WinAPI|✅webview|✅|
|本地化|✅|✅||
|升级助手和下载助手||✅||
|可缩放的显示分辨率|✅|||


### 支持的远程API

| 差异           | QQPilot                     |FishCakeQQ|QQPilotPocketEdition|
|--------------------|---------------------------|-|-|
|Chat Completion API|✅|✅|✅|
|OneBot|需要外接CompletionConnector|✅|你可以试试外接CompletionConnector|
|Ollama|✅|✅||
|内置模型|✅|✅||
|MNN Chat|||✅|


