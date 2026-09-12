using System.Diagnostics;

namespace QQPilot4
{
    internal static class Upload2
    {

        public static void UploadImage((int, int, int, int) uploadImagePossibleActualSize)
        {
            Vision.Screenshot(uploadImagePossibleActualSize);

            Thread.Sleep(2000);

            List<(uint x, uint y)> copyButtonPosition = Vision.FindTemplates("screenshot.png", "uploadImage.png", 30, 1);
            if (copyButtonPosition.Count <= 0)
            {
                Log.Print("使用模板匹配查找上传图片按钮失败");
                //DockLog.Log2("使用模板匹配查找上传图片按钮失败");

                Process.Start("uploadImage2.exe").WaitForExit();
                Thread.Sleep(200);
                GUIOperation.HotKey("ctrl", "v");

            }
            else
            {
                var (x, y) = copyButtonPosition[0];
                x += (uint)uploadImagePossibleActualSize.Item1;
                y += (uint)uploadImagePossibleActualSize.Item2;
                GUIOperation.Click((int)x, (int)y);
                Thread.Sleep(4000);
                Upload.upload();
            }
            Thread.Sleep(4000);
        }
        public static void UploadSelectedImage((int, int, int, int) uploadImagePossibleActualSize,string file)
        {
            Vision.Screenshot(uploadImagePossibleActualSize);

            Thread.Sleep(2000);

            List<(uint x, uint y)> copyButtonPosition = Vision.FindTemplates("screenshot.png", "uploadImage.png", 30, 1);
            if (copyButtonPosition.Count <= 0)
            {
                Log.Print("使用模板匹配查找上传图片按钮失败");
                //DockLog.Log2("使用模板匹配查找上传图片按钮失败");

                Process.Start("uploadImage2.exe").WaitForExit();
                Thread.Sleep(200);
                GUIOperation.HotKey("ctrl", "v");

            }
            else
            {
                var (x, y) = copyButtonPosition[0];
                x += (uint)uploadImagePossibleActualSize.Item1;
                y += (uint)uploadImagePossibleActualSize.Item2;
                GUIOperation.Click((int)x, (int)y);
                Thread.Sleep(4000);
                GUIOperation.UploadSelectedFile(file);
            }
            Thread.Sleep(4000);
        }
    }
}