using IniParser.Model;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using TextCopy;
using static System.Net.Mime.MediaTypeNames;

namespace QQPilot4
{
    internal class Program
    {
        static bool autoFocusShouldRun = true;
        static readonly bool debug =false;
        public static string characterName="";
        static void Main(string[] args)
        {
            if (Environment.OSVersion.Version.Major >= 10)
            {
                Console.OutputEncoding = Encoding.UTF8;
            }
            //Answer a=new();
            //var g=a.GetAnswer([new("Username1", [], "你好", DateTime.Now.ToShortDateString(), false)]);

            //Console.WriteLine(g.ToString());
            ////Console.WriteLine(g.ToString());
            //var g2 = g.Absolute();
            //GUIOperation.UploadSelectedFile(g2[0]);
            //return;

            //A.GetAnswer(new ChatContent())
            //ConversationStyleExtract.Test();
            //return;
            //DockLog.Init();
            Process? p=null;
            try
            {
                p= Process.Start("ScaleToINI.exe");

            }
            catch (Exception e)
            {
                Log.Print(e.ToString(),Log.Stat.ERROR);
            }
            //GUIOperation.Init();
            //GUIOperation.Click(3, 3);
            ArrowLoad.StartLoading(ConsoleColor.Green, "正在初始化");
            //DockLog.Log2("正在初始化");
            GUIOperation.Init();
            IniParser.FileIniDataParser parser = new();
            IniData ini                 = parser.ReadFile("config.ini", new UTF8Encoding(false));
            KeyDataCollection general   = ini["general"];
            (int, int) size             = (int.Parse(general["width"]), int.Parse(general["height"]));
            p?.WaitForExit();
            float scale                 = float.Parse(general["scale"]);
            int scrollTries             = int.Parse(general["scroll"]);
            bool withImage              = (general["withimage"].Equals("true", StringComparison.CurrentCultureIgnoreCase));
            bool autoLogin              = (general["autologin"].Equals("true", StringComparison.CurrentCultureIgnoreCase));
            int sendImagePossibility    = int.Parse(general["sendimagepossibility"]);
            bool isVisionModel          = (general["isvisionmodel"].Equals("true", StringComparison.CurrentCultureIgnoreCase));
            bool ATDetect               = (general["atdetect"].Equals("true", StringComparison.CurrentCultureIgnoreCase));
            int tapTimes                =  int.Parse(general["tab_times"]);
            characterName               = general["name"];
            characterName ??= "";
            long tokenCount = 0;
            try
            {
                tokenCount=long.Parse(File.ReadAllText("tokencount.txt",new UTF8Encoding(false)));
            }catch
            {
                tokenCount=0;
                File.WriteAllText("tokencount.txt", 0.ToString(), new UTF8Encoding(false));
            }
            Console.ForegroundColor = ConsoleColor.Cyan;
            Log.Print($"QQPilot {general["version"]}");
            ArrowLoad.StopLoading();
            Console.ResetColor();

            //DockLog.Log2("初始化完成");
            Log.Print("初始化完成");

            string OSDescription = System.Runtime.InteropServices.RuntimeInformation.OSDescription;
            Console.ForegroundColor = ConsoleColor.Yellow;
            Log.Print(OSDescription);
            Console.ResetColor();

            Thread autoFocusThread = new(AutoFocus);
            autoFocusThread.Start();

            Log.SetColor(ConsoleColor.Magenta);
            Log.Print("请将消息栏拉到最小!");

            Log.SetColor(ConsoleColor.Cyan);
            Log.Print($"欢迎您 {characterName}。");

            Log.Print("自动聚焦功能已开启");
            if(autoLogin)
            {
                Log.Print("自动登录功能已开启");
                Log.Print("正在尝试登录...");
                //DockLog.Log2("正在尝试登录...");

                for (int i = 0; i < 4; i++)
                {
                    Vision.FullScreenShot();
                    var (x, y) = Vision.ContainsBlue();
                    if(x==0 && y==0)
                    {
                        Thread.Sleep(1000);
                        continue;
                    }
                    GUIOperation.Click((int)x, (int)y);
                    Thread.Sleep(2000);

                }
                Thread.Sleep(1000);
            }
            Log.SetColor(ConsoleColor.Cyan);
            Log.Print("======================================================");
            Log.Print("");
            Log.SetColor(ConsoleColor.Yellow);
            Log.Print("\t使用时请勿移动鼠标！");
            Log.Print("");
            Log.SetColor(ConsoleColor.Cyan);
            Log.Print("======================================================");

            size = ((int)(size.Item1 * scale), (int)(size.Item2 * scale));
            (int,int,int,int) positionRect=(0,0,size.Item1,size.Item2);

            // 聊天列表实际大小
            var chatListActualSize = Positions.ToActualSize(Positions.CHAT_LIST_BBOX_RELATIVE_SIZE, size);
            //Log.Print( $"聊天列表实际大小: {chatListActualSize}");
            // 聊天区域实际大小
            var conversationActualSize = Positions.ToActualSize(Positions.CONVERSATION_BBOX_RELATIVE_SIZE, size);
            //Log.Print( $"聊天区域实际大小: {conversationActualSize}");
            // 输入框实际大小
            var commentSectionActualSize = Positions.ToActualSize(Positions.COMMENT_SECTION_BBOX_RELATIVE_SIZE, size);
            //Log.Print( $"输入框实际大小: {commentSectionActualSize}");
            // 发送按钮实际大小
            var sendButtonActualSize = Positions.ToActualSize(Positions.SEND_BUTTON_BBOX_RELATIVE_SIZE, size);
            //Log.Print( $"发送按钮实际大小: {sendButtonActualSize}");
            // 退出会话按钮实际大小
            var exitConversationActualSize = Positions.ToActualSize(Positions.EXIT_CONVERSATION_BBOX_RELATIVE_SIZE, size);
            //Log.Print( $"退出会话按钮实际大小: {exitConversationActualSize}");
            // 发送图片按钮实际大小
            var sendImageActualSize = Positions.ToActualSize(Positions.SEND_IMAGE_BBOX_RELATIVE_SIZE, size);
            //Log.Print( $"发送图片按钮实际大小: {sendImageActualSize}");
            // @位置实际大小
            var atPlaceActualSize = Positions.ToActualSize(Positions.AT_PLACE_BBOX_RELATIVE_SIZE, size);
            //Log.Print( $"@位置实际大小: {atPlaceActualSize}");
            // 拖拽起止位置
            var startDraggingAbsolutePosition = Positions.ToActualPoint(Positions.START_DRAGGING_RELATIVE_POSITION, size);
            var endDraggingAbsolutePosition = Positions.ToActualPoint(Positions.END_DRAGGING_RELATIVE_POSITION, size);
            //Log.Print( $"开始拖拽位置: {startDraggingAbsolutePosition}");
            //Log.Print( $"结束拖拽位置: {endDraggingAbsolutePosition}");
            // 聊天按钮和联系人按钮位置
            var chatButtonActualPosition = Positions.ToActualPoint(Positions.CHAT_BUTTON_RELATIVE_POSITION, size);
            //Log.Print( $"聊天按钮实际位置: {chatButtonActualPosition}");
            var contactButtonActualPosition = Positions.ToActualPoint(Positions.CONTACT_BUTTON_RELATIVE_POSITION, size);
            //Log.Print( $"联系人按钮实际位置: {contactButtonActualPosition}");
            // 取消按钮位置（未打印日志，按需添加）
            var cancelButtonActualPosition = Positions.ToActualPoint(Positions.CANCEL_BUTTON_RELATIVE_POSITION, size);
            // 上传图片和复制按钮可能区域
            var uploadImagePossibleActualSize = Positions.ToActualSize(Positions.UPLOAD_IMAGE_POSSIBLE_BBOX_RELATIVE_SIZE, size);
            var copyButtonPossibleActualSize = Positions.ToActualSize(Positions.COPY_BUTTON_POSSIBLE_BBOX_RELATIVE_SIZE, size);
            //Log.Print( $"上传图片可能位置: {uploadImagePossibleActualSize}");
            //Log.Print( $"复制按钮可能位置: {copyButtonPossibleActualSize}");
            Answer? answer = null;
            bool cancelled=false;
            Console.CancelKeyPress += (sender, e) =>
            {
                Console.ForegroundColor = ConsoleColor.Red;
                Log.Print("\n结束运行");
                //DockLog.Exit();
                Console.ResetColor();
                // 设置 e.Cancel = true 可以阻止程序立即终止，
                // 允许执行清理逻辑后再退出
                cancelled = true;
                autoFocusShouldRun = false;
                autoFocusThread.Join();
                e.Cancel = true;
                throw new SystemException("Terminate");
            };

            while (! cancelled)
            {
                Console.Write("正在寻找新信息...\r");
                //DockLog.Log2("正在寻找新信息...");
                var chatList = Vision.FullScreenShot();
                (uint,uint) contain=(0,0);
                if(ATDetect)
                {
                    contain = Vision.ContainsRedDot(Vision.Rect(atPlaceActualSize));
                }
                else
                {
                    contain = Vision.ContainsRedDot(Vision.Rect(chatListActualSize));
                }
                if(contain!=(0,0))
                {
                    Thread.Sleep(500);
                    if (ATDetect)
                    {
                        contain = Vision.ContainsRedDot(Vision.Rect(atPlaceActualSize));
                    }
                    else
                    {
                        contain = Vision.ContainsRedDot(Vision.Rect(chatListActualSize));
                    }
                    if (contain == (0, 0))
                    {
                        continue;
                    }
                    Console.ForegroundColor = ConsoleColor.Green;
                    Log.Print($"发现红点: {contain}");
                    //DockLog.Log2($"发现红点: {contain}");

                    Console.ResetColor();
                    GUIOperation.Click((int)contain.Item1, (int)contain.Item2);
                    Thread.Sleep(1000);
                    //Log.Print(startDraggingAbsolutePosition.Item1.ToString(), startDraggingAbsolutePosition.Item2, endDraggingAbsolutePosition.Item1, endDraggingAbsolutePosition.Item2);
                    GUIOperation.DragFromToSimple(startDraggingAbsolutePosition.Item1, startDraggingAbsolutePosition.Item2, endDraggingAbsolutePosition.Item1, endDraggingAbsolutePosition.Item2);
                    Thread.Sleep(500);
                    GUIOperation.GotoCenter(conversationActualSize);
                    Thread.Sleep(500);

                    Vision.Screenshot(copyButtonPossibleActualSize);
                    Thread.Sleep(1000);
                    Clipboard clipboard = new();
                    clipboard.SetText(string.Empty);


                    List<(uint x, uint y)> points = Vision.FindTemplates("screenshot.png", "./copy.png", 30, 1);
                    if (points.Count == 0)
                    {
                        Log.Print("使用模板匹配查找复制按钮失败");
                        //DockLog.Log2("使用模板匹配查找复制按钮失败");
                        for (int i = 0; i < scrollTries * 2; i++)
                        {
                            Thread.Sleep(400);
                            GUIOperation.ScrollDown(480);

                        }
                        Thread.Sleep(400);

                        GUIOperation.ClickCenter(commentSectionActualSize);

                        for (int i = 0; i < tapTimes; i++)
                        {

                            GUIOperation.Tab();
                            Thread.Sleep(400);

                        }
                        GUIOperation.PressKey("enter");
                        Thread.Sleep(200);

                    }
                    else
                    {
                        Thread.Sleep(800);
                        GUIOperation.Click((int)(points[0].x + copyButtonPossibleActualSize.Item1), (int)(points[0].y + copyButtonPossibleActualSize.Item2));
                        Thread.Sleep(1500);


                    }


                    string chatContentStr = clipboard.GetText() ?? "";
                    if (chatContentStr.Length == 0)
                    {
                        Log.Print("没有提取到消息。", Log.Stat.ERROR);

                        SpinnerLoad.Stop();

                        GoBack(scale, chatButtonActualPosition, contactButtonActualPosition, copyButtonPossibleActualSize, uploadImagePossibleActualSize);
                        continue;
                    }

                    List<ChatContent> ChatContents = ConversationStyleExtract.ParseChatLog(chatContentStr, characterName);
                    SpinnerLoad.Start(ConsoleColor.Green, "等待语言模型生成答案");
                    //DockLog.Log2("等待语言模型生成答案");

                    GUIOperation.ClickCenter(commentSectionActualSize);

                    
                    answer ??= new();
                    //string result = (( ?? new UploadContent("")).ToString() ?? "");
                    UploadContent results= answer.GetAnswer(ChatContents) ?? new("");
                    string result = results.ToString()??"";
                    List<string> imagesToUpload = results.Absolute();
                    result = result.Length > 500 ? result[..500] : result;
                    if (answer.TotalTokens != 0)
                    {
                        tokenCount += answer.TotalTokens;
                        Log.Print($"累计用量: {tokenCount}");
                        try
                        {
                            File.WriteAllText("tokencount.txt", tokenCount.ToString(), new UTF8Encoding(false));
                        }
                        catch (Exception e)
                        {
                            Log.Print(e.ToString(), Log.Stat.ERROR);
                        }
                    }


                    SpinnerLoad.Stop();

                    result = result.Trim();
                    if (result.Replace("\n\n", "") == "" || result == "")
                    {
                        //SpinnerLoad.Stop();
                        if (withImage && sendImagePossibility > 0)
                        {
                            Log.Print("答案未生成,上传图片", Log.Stat.WARN);
                            UploadImageWithoutSend(uploadImagePossibleActualSize);
                            GUIOperation.HotKey("ctrl", "enter");
                            Thread.Sleep(4000);
                            Log.Print("退出会话");
                        }
                        else
                        {
                            Log.Print("答案未生成,退出会话", Log.Stat.ERROR);

                        }
                        GoBack(scale, chatButtonActualPosition, contactButtonActualPosition, copyButtonPossibleActualSize, uploadImagePossibleActualSize);

                        continue;
                    }
                    SpinnerLoad.Stop();
                    Thread.Sleep(100);
                    GUIOperation.ClickCenter(commentSectionActualSize);

                    GUIOperation.SendText(result, commentSectionActualSize);

                    foreach(var image in imagesToUpload)
                    {
                        Log.Print("上传获取的图片");
                        Upload2.UploadSelectedImage(uploadImagePossibleActualSize, image);
                        Upload.escape();

                    }
                    Random r = new();

                    int poss = ((int)(r.NextInt64() % 100));
                    Log.Print($"概率:{poss}");
                    if (withImage && poss < sendImagePossibility)
                    {

                        Log.Print("上传图片");
                        //DockLog.Log2("上传图片");
                        UploadImageWithoutSend(uploadImagePossibleActualSize);
                        Upload.escape();

                    }
                    Thread.Sleep(4000);
                    Log.Print("发送消息 🎉");
                    //DockLog.Log2("发送消息 🎉");

                    GUIOperation.HotKey("ctrl", "enter");
                    Thread.Sleep(4000);
                    Log.Print("退出会话");
                    ////DockLog.Log2("发送消息 🎉");
                    ClearInputSection();
                    GoBack(scale, chatButtonActualPosition, contactButtonActualPosition, copyButtonPossibleActualSize, uploadImagePossibleActualSize);
                    
                }
                else
                {
                    Thread.Sleep(2000);
                }
            }
            autoFocusShouldRun = false;
            autoFocusThread.Join();

            static void GoBack(float scale, (int, int) chatButtonActualPosition, (int, int) contactButtonActualPosition, (int, int, int, int) copyButtonPossibleActualSize, (int, int, int, int) uploadImagePossibleActualSize)
            {
                //点击聊天第一项退出对话

                List<(uint x, uint y)> pointsOfCopy;
                List<(uint x, uint y)> pointsOfUpload;
                int count = 0;
                //检查是否存在上传图片和复制按钮确保已经退出对话。
                do
                {
                    GUIOperation.Click(chatButtonActualPosition.Item1 + (int)(100 * scale), chatButtonActualPosition.Item2 + (int)(80 * scale));
                    Thread.Sleep(3000);
                    //GUIOperation.Click(contactButtonActualPosition.Item1, contactButtonActualPosition.Item2);
                    //Thread.Sleep(500);
                    //GUIOperation.Click(chatButtonActualPosition.Item1, chatButtonActualPosition.Item2);
                    //Thread.Sleep(1000);
                    if (count++ > 2)
                    {
                        break;
                    }

                    Vision.Screenshot(uploadImagePossibleActualSize);
                    Thread.Sleep(1500);
                    pointsOfUpload = Vision.FindTemplates("screenshot.png", "./uploadImage.png", 30, 1);
                    //Log.Print(pointsOfUpload/.) ?? "||");
                    if (pointsOfUpload.Count != 0)
                    {
                        Log.Print($"({pointsOfUpload[0].x},{pointsOfUpload[0].y})");

                        continue;
                    }

                    Vision.Screenshot(copyButtonPossibleActualSize);
                    Thread.Sleep(1500);
                    pointsOfCopy= Vision.FindTemplates("screenshot.png", "./copy.png", 30, 1);
                    
                    if(pointsOfCopy.Count!=0)
                    {

                        Log.Print($"({pointsOfCopy[0].x},{pointsOfCopy[0].y})");

                        continue; 

                    }
        
                    break;

                }
                while ( true);
            }
        }

        private static void ClearInputSection()
        {
            GUIOperation.HotKey("ctrl", "a");
            Thread.Sleep(200);
            GUIOperation.PressKey("backspace");
            Thread.Sleep(200);
        }

        private static void UploadImageWithoutSend((int, int, int, int) uploadImagePossibleActualSize)
        {
            var imageDir = Path.Combine(System.AppDomain.CurrentDomain.BaseDirectory, "Images");
            Log.Print(imageDir);
            List<string> dirs;
            if (!Path.Exists(imageDir))
            {
                dirs = [];
                Log.Print("没有找到图片目录", Log.Stat.ERROR);

            }
            else
            {
                dirs = [.. Directory.EnumerateFiles(imageDir)];

            }
            bool containsImage = false;


            // 支持的图像扩展名
            var imageExtensions = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
                        {
                            ".jpg", ".jpeg", ".png", ".gif"
                        };

            foreach (string dir in dirs)
            {
                Log.Print(dir);

                if (imageExtensions.Contains(Path.GetExtension(dir)))
                {
                    containsImage = true;
                    Log.Print("Image:" + dir);
                    break;
                }

            }
            if (imageDir is not null && containsImage)
            {
                Upload2.UploadImage(uploadImagePossibleActualSize);
            }

            //return copyButtonPosition;
        }

        static void AutoFocus()
        {
            while(autoFocusShouldRun)
            {
                if(! debug)
                { 
                    GUIOperation.Focus_();
                }
                Thread.Sleep(4000);
            }
        }
    }
}
