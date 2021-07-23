fn main() {
    windows::build! {
        Windows::Win32::UI::WindowsAndMessaging::MessageBoxW,
        Windows::Win32::Foundation::{
            CloseHandle,
            HWND, BOOL,
        },
        Windows::Win32::UI::HiDpi::GetDpiForWindow,
        Windows::Win32::Graphics::Gdi::{UpdateWindow, PtInRect},
        Windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWINDOWATTRIBUTE},
        Windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, GetCursorPos, EnumWindows, IsWindowVisible, GetWindowInfo,
            SendMessageW, GetClassNameW, GetWindowTextLengthW, GetWindowTextW,
            GetWindowRect, GetWindowDisplayAffinity, GetWindowThreadProcessId,
            HWND_TOP, WINDOW_EX_STYLE, HTERROR, HTNOWHERE, HTTRANSPARENT, WM_NCHITTEST,
            WINDOW_DISPLAY_AFFINITY,
        },
        Windows::Win32::UI::KeyboardAndMouseInput::EnableWindow,
        Windows::UI::Xaml::Hosting::{DesktopWindowXamlSource, WindowsXamlManager},
        Windows::UI::Xaml::Controls::{
            Panel, RelativePanel, TextBlock, Button, UIElementCollection, ProgressRing,
        },
        Windows::Win32::System::WinRT::{IDesktopWindowXamlSourceNative, IDesktopWindowXamlSourceNative2},
        Windows::UI::Xaml::Media::{SolidColorBrush, VisualTreeHelper},
        Windows::UI::Xaml::Input::{PointerEventHandler, PointerRoutedEventArgs, Pointer},
        Windows::UI::Xaml::Shapes::*,
        Windows::UI::Xaml::FrameworkElement,
        Windows::UI::Colors,
        Windows::UI::Core::{
            CoreWindow, CoreCursor, CoreCursorType, CoreWindow, DispatchedHandler,
            CoreDispatcher, CoreDispatcherPriority,
        },
        Windows::Foundation::Collections::IVectorView,
        Windows::UI::Input::PointerPoint,
        Windows::Win32::System::Pipes::*,
        Windows::Win32::System::WindowsProgramming::{
            PIPE_ACCESS_OUTBOUND, PIPE_ACCESS_INBOUND, PIPE_ACCESS_DUPLEX,
        },
        Windows::Win32::Storage::FileSystem::{
            WriteFile, Wow64DisableWow64FsRedirection, Wow64RevertWow64FsRedirection,
            FILE_FLAGS_AND_ATTRIBUTES,
        },
        Windows::Win32::System::Diagnostics::Debug::{GetLastError, WIN32_ERROR, IMAGE_FILE_MACHINE},
        Windows::Win32::System::Threading::{IsWow64Process2, OpenProcess},
    }
}
