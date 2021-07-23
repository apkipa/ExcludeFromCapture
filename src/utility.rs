use bindings::{
    Windows::Foundation::Point,
    Windows::Win32::Foundation::{
        CloseHandle, BOOL, HANDLE, HWND, LPARAM, POINT, PWSTR, RECT, WPARAM,
    },
    Windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED},
    Windows::Win32::Graphics::Gdi::PtInRect,
    Windows::Win32::Storage::FileSystem::{
        Wow64DisableWow64FsRedirection, Wow64RevertWow64FsRedirection, WriteFile,
        FILE_FLAG_FIRST_PIPE_INSTANCE,
    },
    Windows::Win32::System::Diagnostics::Debug::{
        GetLastError, ERROR_PIPE_CONNECTED, IMAGE_FILE_MACHINE, IMAGE_FILE_MACHINE_AMD64,
        IMAGE_FILE_MACHINE_I386, IMAGE_FILE_MACHINE_UNKNOWN,
    },
    Windows::Win32::System::Pipes::*,
    Windows::Win32::System::Threading::{
        IsWow64Process2, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    },
    Windows::Win32::System::WindowsProgramming::PIPE_ACCESS_OUTBOUND,
    Windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetCursorPos, GetWindowDisplayAffinity, GetWindowInfo,
        GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible, SendMessageW, HTERROR, HTNOWHERE, HTTRANSPARENT, WDA_EXCLUDEFROMCAPTURE,
        WINDOWINFO, WINDOW_DISPLAY_AFFINITY, WM_NCHITTEST, WS_EX_TOOLWINDOW,
    },
    Windows::UI::Core::{CoreCursor, CoreWindow},
    Windows::UI::Xaml::Controls::Panel,
    Windows::UI::Xaml::Media::VisualTreeHelper,
    Windows::UI::Xaml::{FrameworkElement, Input::Pointer, UIElement},
};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::mem::MaybeUninit;
use std::os::windows::ffi::OsStrExt;
use std::ptr::addr_of_mut;
use std::{alloc, fmt, fs, mem, process, ptr, slice, thread, time};
use windows::{Interface, HSTRING};
use winit::window::Window;

// Defer resource releasing helper (ugly implementation)
struct CloseHandleHelper(HANDLE);

impl Drop for CloseHandleHelper {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

impl fmt::Debug for CloseHandleHelper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

struct EnumWindowsProcParam {
    pt: POINT,
    target_hwnd: HWND,
}

fn use_dyn_uninit_u16_slice_with<T>(
    buf_len: usize,
    f: impl FnOnce(&mut [MaybeUninit<u16>]) -> T,
) -> Option<T> {
    unsafe {
        let mem_layout = alloc::Layout::from_size_align(
            mem::size_of::<u16>().checked_mul(buf_len)?,
            mem::align_of::<u16>(),
        )
        .ok()?;
        let str_buf = alloc::alloc(mem_layout);
        if str_buf.is_null() {
            return None;
        }
        let ret_val = Some(f(slice::from_raw_parts_mut(
            str_buf as *mut MaybeUninit<u16>,
            buf_len,
        )));
        alloc::dealloc(str_buf, mem_layout);
        ret_val
    }
}

pub fn get_hwnd_from_window(window: &Window) -> HWND {
    let hwnd = match window.raw_window_handle() {
        RawWindowHandle::Windows(window_handle) => window_handle.hwnd,
        _ => unreachable!("Must be run on Windows"),
    };
    // let hwnd = unsafe { mem::transmute::<_, HWND>(hwnd) };
    // hwnd
    HWND(hwnd as _)
}

pub fn get_element_pointer_capture_count(elem: &FrameworkElement) -> u32 {
    if let Ok(vec) = elem.PointerCaptures() {
        if let Ok(size) = vec.Size() {
            size
        } else {
            0
        }
    } else {
        0
    }
}

pub fn is_pointer_captured_by_element(elem: &FrameworkElement, pointer: &Pointer) -> bool {
    if let Ok(vec) = elem.PointerCaptures() {
        vec.into_iter()
            .any(|i| match (i.PointerId(), pointer.PointerId()) {
                (Ok(id1), Ok(id2)) => id1 == id2,
                _ => false,
            })
    } else {
        false
    }
}

pub fn is_point_in_element(pt: &Point, elem: &UIElement) -> bool {
    if let Ok(iter) = VisualTreeHelper::FindElementsInHostCoordinatesPoint(pt, elem) {
        iter.into_iter().next().is_some()
    } else {
        false
    }
}

pub fn str_to_u16_vec_no_trailing_zero(a: &str) -> Vec<u16> {
    std::ffi::OsStr::new(a).encode_wide().collect()
}

pub fn str_to_u16_vec(a: &str) -> Vec<u16> {
    std::ffi::OsStr::new(a)
        .encode_wide()
        .chain(Some(0))
        .collect()
}

pub fn u16_vec_to_pwstr(a: &mut Vec<u16>) -> PWSTR {
    PWSTR(a.as_mut_ptr())
}

// From: https://stackoverflow.com/a/57503105
// unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
//     const BUFSIZE: i32 = 64;
//     let mut buf_wnd_class_name = MaybeUninit::<[u16; BUFSIZE]>::uninit();
//     let buf_wnd_class_name = buf_wnd_class_name.as_mut_ptr();
//     let mut uwp_app_wnd_class_name = str_to_u16_vec_no_trailing_zero("ApplicationFrameWindow");
//     // let uwp_app_wnd_class_name = u16_vec_to_pwstr(&mut uwp_app_wnd_class);
//     let hwnd_desktop = unsafe { GetDesktopWindow() };
//     let hwnd_parent = unsafe { HWND(GetWindowLongPtrW(hwnd, GWLP_HWNDPARENT)) };
//     let cloaked = DWM_NORMAL_APP_NOT_CLOAKED;

//     let buf_wnd_class_name_len = unsafe { GetClassNameW(hwnd, buf_wnd_class_name, BUFSIZE) };
//     if CompareStringEx(
//         LOCALE_NAME_INVARIANT,
//         0,
//         buf_wnd_class_name,
//         buf_wnd_class_name_len,
//         u16_vec_to_pwstr(&mut uwp_app_wnd_class),
//         uwp_app_wnd_class_name.len(),
//         ptr::null_mut(),
//         ptr::null_mut(),
//         LPARAM(0),
//     ) == CSTR_EQUAL
//     {
//         // ...
//     }
//     // ...
// }

fn is_possible_target_window(hwnd: HWND) -> bool {
    const DWM_NORMAL_APP_NOT_CLOAKED: u32 = 8;
    const DWM_NOT_CLOAKED: u32 = 0;
    unsafe {
        // 1)
        if IsWindowVisible(hwnd) == false {
            return false;
        }

        // 2)
        let mut window_info = MaybeUninit::<WINDOWINFO>::uninit();
        addr_of_mut!((*window_info.as_mut_ptr()).cbSize).write(mem::size_of::<WINDOWINFO>() as _);
        if GetWindowInfo(hwnd, window_info.as_mut_ptr()) == false {
            return false;
        }
        let window_info = window_info.assume_init();
        if (window_info.dwExStyle & WS_EX_TOOLWINDOW.0) != 0 {
            return false;
        }

        // 3)
        let mut cloaked = MaybeUninit::<u32>::uninit();
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED.0 as _,
            cloaked.as_mut_ptr() as _,
            mem::size_of::<u32>() as _,
        )
        .is_err()
        {
            return false;
        }
        let cloaked = cloaked.assume_init();
        match cloaked {
            DWM_NOT_CLOAKED | DWM_NORMAL_APP_NOT_CLOAKED => (),
            _ => return false,
        }
    }

    // Ok
    true
}

fn make_dword(lo: u16, hi: u16) -> u32 {
    ((lo as u32 & 0xffff) | ((hi as u32 & 0xffff) << 16)) as _
}

fn make_lparam(lo: u16, hi: u16) -> LPARAM {
    LPARAM(make_dword(lo, hi) as _)
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    const HTNOWHERE: i32 = bindings::Windows::Win32::UI::WindowsAndMessaging::HTNOWHERE as _;
    let mut window_rect = MaybeUninit::<RECT>::uninit();

    if !is_possible_target_window(hwnd) {
        return true.into();
    }

    if GetWindowRect(hwnd, window_rect.as_mut_ptr()) == false {
        return true.into();
    }

    let params: *mut EnumWindowsProcParam = lparam.0 as _;
    let pt = (*params).pt;

    if PtInRect(window_rect.as_ptr(), &pt) == false {
        return true.into();
    }

    match SendMessageW(
        hwnd,
        WM_NCHITTEST,
        WPARAM::NULL,
        make_lparam(pt.x as _, pt.y as _),
    )
    .0
    {
        HTERROR | HTNOWHERE | HTTRANSPARENT => return true.into(),
        _ => (*params).target_hwnd = hwnd,
    }

    // Stop further enumeration
    false.into()
}

pub fn find_target_window_from_point(pt: &POINT) -> HWND {
    // let mut params = MaybeUninit::<EnumWindowsProcParam>::uninit();
    // let params = unsafe {
    //     addr_of_mut!((*params.as_mut_ptr()).pt).write(*pt);
    //     addr_of_mut!((*params.as_mut_ptr()).target_hwnd).write(HWND::NULL);
    //     EnumWindows(Some(enum_windows_proc), LPARAM(params.as_mut_ptr() as _));
    //     params.assume_init()
    // };
    // params.target_hwnd
    let mut params = EnumWindowsProcParam {
        pt: *pt,
        target_hwnd: HWND::NULL,
    };
    unsafe {
        EnumWindows(Some(enum_windows_proc), LPARAM(&mut params as *mut _ as _));
    }
    params.target_hwnd
}

pub fn get_class_name_as_hstring(hwnd: HWND) -> HSTRING {
    const BUFLEN: i32 = 256 + 1;
    // let mut str_buf = MaybeUninit::<[u16; BUFLEN as _]>::uninit();
    // let str_buf_ptr = (*str_buf.as_mut_ptr()).as_mut_ptr();
    let mut str_buf: [MaybeUninit<u16>; BUFLEN as _] =
        unsafe { MaybeUninit::uninit().assume_init() };
    let str_buf_ptr = str_buf.as_mut_ptr() as _;
    unsafe {
        match GetClassNameW(hwnd, PWSTR(str_buf_ptr), BUFLEN) {
            0 => HSTRING::new(),
            len => HSTRING::from_wide(slice::from_raw_parts(str_buf_ptr, len as _)),
        }
    }
}

pub fn get_window_text_as_hstring(hwnd: HWND) -> HSTRING {
    let buf_len = match unsafe { GetWindowTextLengthW(hwnd) } {
        0 => return HSTRING::new(),
        len => len + 1,
    };

    use_dyn_uninit_u16_slice_with(buf_len as _, |s| unsafe {
        match GetWindowTextW(hwnd, PWSTR(s.as_mut_ptr() as _), buf_len) {
            0 => HSTRING::new(),
            len => HSTRING::from_wide(mem::transmute(&s[..len as _])),
        }
    })
    .unwrap_or_else(|| HSTRING::new())
}

// element.FindName() is almost impossible to use in UWP apps, manual search is required
pub fn find_name_in_panel(panel: &Panel, name: &str) -> windows::Result<FrameworkElement> {
    // let err = || windows::HRESULT(0xc000027b).into();
    let err = || windows::HRESULT(0x80004003).into();
    if panel.Name()? == name {
        return panel.cast();
    }
    panel
        .Children()?
        .into_iter()
        .filter_map(|i| i.cast::<FrameworkElement>().ok())
        .find(|i| i.Name().map_or(false, |i| name == i))
        .ok_or_else(err)
}

pub fn get_cursor_pos() -> POINT {
    let mut cursor_pos = MaybeUninit::<POINT>::uninit();
    unsafe {
        let p = cursor_pos.as_mut_ptr();
        if GetCursorPos(p) == false {
            // Failed to retrieve cursor position, use (0, 0) instead
            p.write(POINT { x: 0, y: 0 });
        }
        cursor_pos.assume_init()
    }
}

pub fn set_core_window_cursor(cur: &CoreCursor) -> windows::Result<()> {
    CoreWindow::GetForCurrentThread()?.SetPointerCursor(cur)
}

#[derive(Clone, Copy)]
struct SysProcArchGroup {
    sys: IMAGE_FILE_MACHINE,
    proc: IMAGE_FILE_MACHINE,
}

fn arch_from_process(pid: u32) -> Option<SysProcArchGroup> {
    let proc_handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) };
    let mut proc_arch = MaybeUninit::<u16>::uninit();
    let mut native_arch = MaybeUninit::<u16>::uninit();
    if proc_handle.is_null() {
        return None;
    }
    let _defer_res_release_helper = CloseHandleHelper(proc_handle);

    if unsafe {
        IsWow64Process2(
            proc_handle,
            proc_arch.as_mut_ptr(),
            native_arch.as_mut_ptr(),
        )
    } == false
    {
        return None;
    }

    // drop(_defer_res_release_helper);

    Some(SysProcArchGroup {
        sys: IMAGE_FILE_MACHINE(unsafe { native_arch.assume_init() }),
        proc: IMAGE_FILE_MACHINE(unsafe { proc_arch.assume_init() }),
    })
}

fn mavinject_path_from_arch(arch: SysProcArchGroup) -> Option<&'static str> {
    match (arch.sys, arch.proc) {
        (IMAGE_FILE_MACHINE_I386, _) | (IMAGE_FILE_MACHINE_AMD64, IMAGE_FILE_MACHINE_UNKNOWN) => {
            Some("C:\\Windows\\System32\\mavinject.exe")
        }
        (IMAGE_FILE_MACHINE_AMD64, IMAGE_FILE_MACHINE_I386) => {
            Some("C:\\Windows\\SysWOW64\\mavinject.exe")
        }
        _ => None,
    }
}

fn dll_name_from_arch(arch: SysProcArchGroup) -> Option<&'static str> {
    let proc_real_arch = match (arch.sys, arch.proc) {
        (arch, IMAGE_FILE_MACHINE_UNKNOWN) => arch,
        (_, arch) => arch,
    };
    match proc_real_arch {
        IMAGE_FILE_MACHINE_I386 => Some("dllsub.x86.dll"),
        IMAGE_FILE_MACHINE_AMD64 => Some("dllsub.x64.dll"),
        _ => None,
    }
}

pub fn is_window_hidden_from_capture(hwnd: HWND) -> bool {
    // This function assumes that the host system is Windows 10 Version 2004
    let mut final_affinity = MaybeUninit::<u32>::uninit();
    if unsafe { GetWindowDisplayAffinity(hwnd, final_affinity.as_mut_ptr()) } == false {
        return false;
    }
    let final_affinity = unsafe { final_affinity.assume_init() };
    WINDOW_DISPLAY_AFFINITY(final_affinity) == WDA_EXCLUDEFROMCAPTURE
}

pub fn hide_window_from_capture(hwnd: HWND) -> bool {
    // Retrieve target pid & dll name
    let mut target_pid = 0; // If call failed, target_pid will be 0
    unsafe {
        GetWindowThreadProcessId(hwnd, addr_of_mut!(target_pid));
    }
    let arch = match arch_from_process(target_pid) {
        Some(arch) => arch,
        None => return false,
    };
    let target_dll_name = match dll_name_from_arch(arch) {
        Some(name) => name,
        None => return false,
    };
    let canonicalized_target_dll = match fs::canonicalize(target_dll_name) {
        Ok(v) => v,
        _ => return false,
    };
    let target_mavinject_name = match mavinject_path_from_arch(arch) {
        Some(name) => name,
        None => return false,
    };

    // Create pipe
    #[repr(C)]
    struct PipeDllParams {
        target_hwnd: HWND,
        affinity: u32,
    }
    let mut params = PipeDllParams {
        target_hwnd: hwnd,
        affinity: WDA_EXCLUDEFROMCAPTURE.0,
    };
    let pipe_handle = unsafe {
        let mut pipe_name = str_to_u16_vec("\\\\.\\pipe\\excludefromcapture_pipedlldata");
        let pipe_name = u16_vec_to_pwstr(&mut pipe_name);
        CreateNamedPipeW(
            pipe_name,
            PIPE_ACCESS_OUTBOUND | FILE_FLAG_FIRST_PIPE_INSTANCE.0,
            PIPE_TYPE_MESSAGE.0 | PIPE_READMODE_MESSAGE.0 | PIPE_NOWAIT.0,
            1,
            mem::size_of::<PipeDllParams>() as _,
            0,
            0,
            ptr::null_mut(),
        )
    };
    if pipe_handle.is_invalid() {
        return false;
    }
    let _defer_res_release_helper = CloseHandleHelper(pipe_handle);

    // Create process for injection
    let mut redir_data = MaybeUninit::uninit();
    if unsafe { Wow64DisableWow64FsRedirection(redir_data.as_mut_ptr()) } == false {
        return false;
    }
    let redir_data = unsafe { redir_data.assume_init() };
    let status = process::Command::new(target_mavinject_name)
        .arg(target_pid.to_string())
        .arg("/INJECTRUNNING")
        .arg(canonicalized_target_dll)
        .status();
    unsafe {
        Wow64RevertWow64FsRedirection(redir_data);
    }
    if status.is_err() {
        return false;
    }

    // Wait for DLL to connect
    unsafe {
        if !(0..30).any(|_| {
            thread::sleep(time::Duration::from_millis(100));
            if ConnectNamedPipe(pipe_handle, ptr::null_mut()) != false {
                return false;
            }
            if GetLastError() != ERROR_PIPE_CONNECTED {
                return false;
            }
            true
        }) {
            return false;
        }
    }

    // Pass data to DLL
    unsafe {
        if WriteFile(
            pipe_handle,
            addr_of_mut!(params) as _,
            mem::size_of::<PipeDllParams>() as _,
            MaybeUninit::uninit().as_mut_ptr(),
            ptr::null_mut(),
        ) == false
        {
            return false;
        }
    }
    drop(_defer_res_release_helper);

    thread::sleep(time::Duration::from_secs(3));

    // Check for final affinity
    is_window_hidden_from_capture(hwnd)
}
