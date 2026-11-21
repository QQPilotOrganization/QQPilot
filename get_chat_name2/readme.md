## getChatName2
适用于Windows8.1+和没有或无法安装OCR功能的提取聊天名称的工具。

你**额外**需要getChatName.ini:
```ini
[general]
modelName=AnyOCR
serverUrl=AnyUrl
apiKey=Key
```
>apiKey设置为`!INENV!`后程序会从`GET_CHAT_NAME_API_KEY`环境变量获取API-Key


