using IniParser;
using IniParser.Model;
using Microsoft.VisualBasic;
using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows;
using TextCopy;
//using Xamarin.Essentials;

namespace QQPilot4
{
    class GUIOperation
    {
        // === 常量 ===
        private const int WHEEL_DELTA = 120;
        private const string DLL_NAME = "InputEvent.dll";
        private const string CONFIG = "config.ini";
        private const string ConfigPath = CONFIG;

        // === 全局变量 ===
        private static IntPtr _libHandle = IntPtr.Zero;
        private static bool _dllLoaded = false;

        // === 配置项 ===
        private static int ScrollCount = 1; // 默认值
        private static bool AutoFocusing = false;

        // === DLL 函数委托定义 ===
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate bool MousegotoDelegate(uint x, uint y);


        [DllImport("InputEvent.dll", CallingConvention = CallingConvention.Cdecl)]
        public static extern bool SmoothMousegoto(uint x, uint y);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate bool LclickDelegate(uint x, uint y);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate bool DragFromToDelegate(uint x1, uint y1, uint x2, uint y2, float duration);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate bool ScrollUpDownDelegate(int delta);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate bool ScrollLeftRightDelegate(int delta);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate ushort GetVkKeyDelegate(byte[] keyName);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate bool SimpleActionDelegate();

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate bool HotKeyDelegate(ushort modVk, ushort keyVk);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate bool PressKeyDelegate(ushort vk);

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate bool DPIAwarenessPrologueDelegate();

        // === 函数指针缓存 ===
        private static MousegotoDelegate _mouseGoto;
        //private static SmoothMousegotoDelegate _smoothMouseGoto;
        private static LclickDelegate _lClick;
        private static DragFromToDelegate _dragFromTo;
        private static ScrollUpDownDelegate _scrollUp, _scrollDown;
        private static ScrollLeftRightDelegate _scrollLeft, _scrollRight;
        private static GetVkKeyDelegate _getVkKey;
        private static SimpleActionDelegate _copy, _paste, _selectAll, _undo;
        private static HotKeyDelegate _hotKey;
        private static PressKeyDelegate _press;
        private static DPIAwarenessPrologueDelegate _dpiAwareness;

        // === 加载 DLL ===
        private static void LoadDll()
        {
            if (_dllLoaded) return;

            string dllPath = null;
            string[] searchPaths = { Directory.GetCurrentDirectory(), AppDomain.CurrentDomain.BaseDirectory };
            foreach (var path in searchPaths)
            {
                var candidate = Path.Combine(path, DLL_NAME);
                if (File.Exists(candidate))
                {
                    dllPath = candidate;
                    break;
                }
            }

            if (string.IsNullOrEmpty(dllPath))
                throw new FileNotFoundException($"找不到 {DLL_NAME}，请确保它在当前目录或系统 PATH 中");

            _libHandle = LoadLibrary(dllPath);
            if (_libHandle == IntPtr.Zero)
                throw new DllNotFoundException($"无法加载 DLL: {dllPath}");

            // 获取函数地址并创建委托
            _mouseGoto = Marshal.GetDelegateForFunctionPointer<MousegotoDelegate>(GetProcAddress(_libHandle, "Mousegoto"));
            _lClick = Marshal.GetDelegateForFunctionPointer<LclickDelegate>(GetProcAddress(_libHandle, "Lclick"));
            _dragFromTo = Marshal.GetDelegateForFunctionPointer<DragFromToDelegate>(GetProcAddress(_libHandle, "dragFromTo"));
            _scrollUp = Marshal.GetDelegateForFunctionPointer<ScrollUpDownDelegate>(GetProcAddress(_libHandle, "scrollUp"));
            _scrollDown = Marshal.GetDelegateForFunctionPointer<ScrollUpDownDelegate>(GetProcAddress(_libHandle, "scrollDown"));
            _scrollLeft = Marshal.GetDelegateForFunctionPointer<ScrollLeftRightDelegate>(GetProcAddress(_libHandle, "scrollLeft"));
            _scrollRight = Marshal.GetDelegateForFunctionPointer<ScrollLeftRightDelegate>(GetProcAddress(_libHandle, "scrollRight"));
            _getVkKey = Marshal.GetDelegateForFunctionPointer<GetVkKeyDelegate>(GetProcAddress(_libHandle, "getVkKey"));
            _copy = Marshal.GetDelegateForFunctionPointer<SimpleActionDelegate>(GetProcAddress(_libHandle, "copy"));
            _paste = Marshal.GetDelegateForFunctionPointer<SimpleActionDelegate>(GetProcAddress(_libHandle, "paste"));
            _selectAll = Marshal.GetDelegateForFunctionPointer<SimpleActionDelegate>(GetProcAddress(_libHandle, "selectAll"));
            _undo = Marshal.GetDelegateForFunctionPointer<SimpleActionDelegate>(GetProcAddress(_libHandle, "undo"));
            _hotKey = Marshal.GetDelegateForFunctionPointer<HotKeyDelegate>(GetProcAddress(_libHandle, "hotKey"));
            _press = Marshal.GetDelegateForFunctionPointer<PressKeyDelegate>(GetProcAddress(_libHandle, "press"));
            _dpiAwareness = Marshal.GetDelegateForFunctionPointer<DPIAwarenessPrologueDelegate>(GetProcAddress(_libHandle, "DPIAwarenessPrologue"));

            _dllLoaded = true;
        }

        // === P/Invoke for loading DLLs ===
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern IntPtr LoadLibrary(string lpFileName);

        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern IntPtr GetProcAddress(IntPtr hModule, string lpProcName);



        [DllImport("uploadFile.dll", CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
        private static extern int uploadSelectedImage(string src);
        // === 初始化 ===
        public static bool Init()
        {
            LoadDll();
            IniParser.FileIniDataParser parser = new();
            //autoFocusing=config.getboolean('general','autoFocusing')
            var ini = parser.ReadFile(CONFIG);
            ScrollCount = int.Parse(ini["general"]["scroll"]);
            autoFocusing = (ini["general"]["autofocusing"].Equals("true", StringComparison.CurrentCultureIgnoreCase))!;
            bool success = _dpiAwareness();
            if (!success)
                Console.WriteLine("⚠️ 警告: DPI 感知设置失败（可能影响高分屏坐标精度）");
            return success;
        }

        // === 封装函数 ===
        public static bool MouseMove(int x, int y) => _mouseGoto((uint)x, (uint)y);
        public static bool Click(int x, int y){
            SmoothMousegoto((uint)x, (uint)y);
           return  _lClick((uint) x, (uint) y);
        
        }
        public static bool ClickCenter((int,int,int,int) area)
        {
            var (x, y) = getAreaCenter(area);
            return Click(x, y);
        }

        public static bool DragFromTo(int x1, int y1, int x2, int y2, float duration = 0.1f) =>
            _dragFromTo((uint)x1, (uint)y1, (uint)x2, (uint)y2, duration);

        public static bool ScrollUp(int delta = WHEEL_DELTA) => _scrollUp(delta);
        public static bool ScrollDown(int delta = WHEEL_DELTA) => _scrollDown(delta);
        public static bool ScrollLeft(int delta = WHEEL_DELTA) => _scrollLeft(delta);
        public static bool ScrollRight(int delta = WHEEL_DELTA) => _scrollRight(delta);

        public static bool Copy() => _copy();
        public static bool Paste() => _paste();
        public static bool SelectAll() => _selectAll();
        public static bool Undo() => _undo();

        public static bool PressKey(string keyName)
        {
            var bytes = System.Text.Encoding.ASCII.GetBytes(keyName.ToUpperInvariant());
            ushort vk = _getVkKey(bytes);
            if (vk == 0) throw new ArgumentException($"未知键名: {keyName}");
            return _press(vk);
        }

        public static bool HotKey(string modifier, string key)
        {
            modifier = modifier.ToUpperInvariant();
            key = key.ToUpperInvariant();

            var modBytes = System.Text.Encoding.ASCII.GetBytes(modifier);
            var keyBytes = System.Text.Encoding.ASCII.GetBytes(key);

            ushort modVk = _getVkKey(modBytes);
            ushort keyVk = _getVkKey(keyBytes);

            if (modVk == 0) throw new ArgumentException($"未知修饰键: {modifier}");
            if (keyVk == 0) throw new ArgumentException($"未知按键: {key}");

            return _hotKey(modVk, keyVk);
        }

        // === 鼠标按下/抬起（需 DLL 支持）===
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate bool MouseUpDownDelegate();
        private static MouseUpDownDelegate _mouseDown, _mouseUp;



        public static bool MouseDown()
        {
            if (_mouseDown == null)
                _mouseDown = Marshal.GetDelegateForFunctionPointer<MouseUpDownDelegate>(GetProcAddress(_libHandle, "LmouseDown"));
            return _mouseDown();
        }

        public static bool MouseUp()
        {
            if (_mouseUp == null)
                _mouseUp = Marshal.GetDelegateForFunctionPointer<MouseUpDownDelegate>(GetProcAddress(_libHandle, "LmouseUp"));
            return _mouseUp();
        }

        // === 辅助函数 ===
        public static void Tab() => PressKey("TAB");

        public static void UploadFile()
        {
            string uploadDllPath = Path.GetFullPath("uploadFile.dll");
            IntPtr uploadLib = LoadLibrary(uploadDllPath);
            if (uploadLib == IntPtr.Zero)
            {
                Console.WriteLine("无法加载 uploadFile.dll");
                return;
            }

            var uploadFunc = Marshal.GetDelegateForFunctionPointer<UploadDelegate>(GetProcAddress(uploadLib, "upload"));
            int result = uploadFunc();
            if (result != 0)
            {
                Console.WriteLine("Upload failed");
                Thread.Sleep(500);
                PressKey("ESC");
            }
            Console.WriteLine();
        }

        public static void UploadSelectedFile(string file)
        {

            uploadSelectedImage(file);
        }

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        private delegate int UploadDelegate();

        static bool? autoFocusing;
        public static void Focus_()
        {

            Focus.focus((bool)(autoFocusing??false));
        }

        public static void ScrollUpBatch(int length = 120)
        {
            for (int i = 0; i < ScrollCount; i++)
            {
                ScrollUp(length);
                Thread.Sleep(100);
            }
        }

        public static void ScrollDownBatch(int length = 240)
        {
            for (int i = 0; i < ScrollCount; i++)
            {
                ScrollDown(length);
                Thread.Sleep(100);
            }
        }

        public static void Goto(int x, int y)
        {
            SmoothMousegoto((uint)x, (uint)y);
               
        
        }
        public static (int,int) getAreaCenter((int, int, int, int) area)
        {
            int pos1 = area.Item1 + ((area.Item3 - area.Item1) % 2);
            int pos2 = area.Item2 + ((area.Item4 - area.Item2) % 2);
            return (pos1, pos2);
        }
        public static void GotoCenter((int,int,int,int) area)
        {
            var (pos1, pos2) = getAreaCenter(area);
            Goto(pos1, pos2);
        }
        public static void SendText(string text, (int, int, int, int) commentSectionActualSize)
        {
            Log.SetColor(ConsoleColor.Green);
            Log.Print("发消息->" + text);
            string[] result=text.Split("[[NEXT]]");
            //Log.Print($"{result}");

            foreach (var item in result)
            {
                Log.Print(item + ";");

                string[] inputs = item.Split('\n');
                foreach (var item1 in inputs.Take(inputs.Length - 1))
                {

                    PasteTextToSection(item1, commentSectionActualSize);


                    ;
                    PressKey("ENTER");
                    Thread.Sleep(200);


                }

                PasteTextToSection(inputs.Last(), commentSectionActualSize);


                //Thread.Sleep(200);
                ////HotKey("ctrl", "v");
                //Thread.Sleep(1000);
                ClickCenter(commentSectionActualSize);
                Thread.Sleep(200);
                HotKey("ctrl", "enter");
            }   


        }

        private static void PasteTextToSection(string text, (int, int, int, int) section)
        {
            ClipboardService.SetText(text);
            Thread.Sleep(200);
            ClickCenter(section);
            HotKey("ctrl", "v");
            Thread.Sleep(800);
        }

        public static void DragFromToSimple(int x1, int y1, int x2, int y2)
        {
            SmoothMousegoto((uint)x1, (uint)y1);
            //MouseMove(x1, y1);
            Thread.Sleep(100);
            MouseDown();
            Thread.Sleep(100);
            //MouseMove(x2, y2);
            SmoothMousegoto((uint)x2, (uint)y2);

            Thread.Sleep(ScrollCount * 1000); 
            MouseUp();
        }
    }
}