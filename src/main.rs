#![windows_subsystem = "windows"]
// #![allow(non_snake_case)]
// #![allow(unused_imports)]
// #![allow(unused_variables)]
// #![allow(dead_code)]

macro_rules! enclose {
    ( ($( $x:ident ),*) $y:expr ) => {
        {
            $(let $x = $x.clone();)*
            $y
        }
    };
}

use bindings::{
    Windows::Win32::Foundation::HWND,
    Windows::Win32::System::WinRT::IDesktopWindowXamlSourceNative,
    Windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOP, SWP_SHOWWINDOW},
    Windows::UI::Colors,
    Windows::UI::Core::{
        CoreCursor, CoreCursorType, CoreDispatcherPriority, CoreWindow, DispatchedHandler,
    },
    Windows::UI::Xaml::Controls::{Button, ProgressRing, RelativePanel, TextBlock},
    Windows::UI::Xaml::Hosting::DesktopWindowXamlSource,
    Windows::UI::Xaml::Media::SolidColorBrush,
    Windows::UI::Xaml::Shapes::Rectangle,
    Windows::UI::Xaml::{
        FrameworkElement, Input::PointerEventHandler, RoutedEventHandler, TextWrapping, Thickness,
    },
};
use std::convert::TryFrom;
use std::sync::{Arc, RwLock};
use std::thread;
use windows::{IInspectable, Interface};
use winit::{
    dpi::{PhysicalSize, LogicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod utility;

use crate::utility::*;

#[derive(Clone)]
struct CursorsPack {
    cross: CoreCursor,
    hand: CoreCursor,
    no: CoreCursor,
    arrow: CoreCursor,
}

fn get_cursors_pack() -> windows::Result<CursorsPack> {
    Ok(CursorsPack {
        cross: CoreCursor::CreateCursor(CoreCursorType::Cross, 0)?,
        hand: CoreCursor::CreateCursor(CoreCursorType::Hand, 0)?,
        no: CoreCursor::CreateCursor(CoreCursorType::UniversalNo, 0)?,
        arrow: CoreCursor::CreateCursor(CoreCursorType::Arrow, 0)?,
    })
}

#[derive(Clone)]
struct ControlsPack {
    container: RelativePanel,
    text_help: TextBlock,
    btn_hide: Button,
    rectangle: Rectangle,
    rectangle_h: Rectangle,
    rectangle_v: Rectangle,
    text_resp: TextBlock,
    prog_ring: ProgressRing,
}

fn get_controls_pack() -> windows::Result<ControlsPack> {
    Ok(ControlsPack {
        container: RelativePanel::new()?,
        text_help: TextBlock::new()?,
        btn_hide: Button::new()?,
        rectangle: Rectangle::new()?,
        rectangle_h: Rectangle::new()?,
        rectangle_v: Rectangle::new()?,
        text_resp: TextBlock::new()?,
        prog_ring: ProgressRing::new()?,
    })
}

#[derive(Clone)]
struct BrushesPack {
    white: SolidColorBrush,
    blue: SolidColorBrush,
    black: SolidColorBrush,
}

fn get_brushes_pack() -> windows::Result<BrushesPack> {
    let white_brush = SolidColorBrush::new()?;
    white_brush.SetColor(Colors::White()?)?;
    let blue_brush = SolidColorBrush::new()?;
    blue_brush.SetColor(Colors::Blue()?)?;
    let black_brush = SolidColorBrush::new()?;
    black_brush.SetColor(Colors::Black()?)?;

    Ok(BrushesPack {
        white: white_brush,
        blue: blue_brush,
        black: black_brush,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window_inner_size = LogicalSize {
        width: 350,
        height: 400,
    };
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("ExcludeFromCapture")
        .with_inner_size(window_inner_size)
        .with_resizable(false)
        .with_visible(false)
        .build(&event_loop)?;
    let window_inner_size: PhysicalSize<u32> = window.inner_size();

    // Rc seems OK, but we prefer Arc in case of (?) data race
    let stored_hwnd_lock = Arc::new(RwLock::new(HWND::NULL));

    let desktop_source = DesktopWindowXamlSource::new()?;
    let hwnd_window = get_hwnd_from_window(&window);
    let interop: IDesktopWindowXamlSourceNative = desktop_source.cast()?;
    let hwnd_island = unsafe {
        interop.AttachToWindow(hwnd_window)?;
        interop.get_WindowHandle()?
    };
    unsafe {
        SetWindowPos(
            hwnd_island,
            HWND_TOP,
            0,
            0,
            window_inner_size.width as _,
            window_inner_size.height as _,
            SWP_SHOWWINDOW,
        );
    };

    let cursors = get_cursors_pack()?;
    let ctrls = get_controls_pack()?;
    ctrls.container.SetPadding(Thickness {
        Left: 5.0,
        Top: 0.0,
        Right: 5.0,
        Bottom: 0.0,
    })?;
    let brushes = get_brushes_pack()?;
    ctrls.container.SetBackground(&brushes.white)?;
    ctrls
        .text_help
        .SetText("ℹ️ Drag the cross to the target window, then press `Hide window` button.")?;
    ctrls
        .text_help
        .SetTextWrapping(TextWrapping::WrapWholeWords)?;
    ctrls.text_help.SetFontSize(18.0)?;
    RelativePanel::SetBelow(&ctrls.btn_hide, &ctrls.text_help)?;
    ctrls
        .btn_hide
        .SetContent(IInspectable::try_from("Hide window")?)?;
    ctrls.btn_hide.SetFontSize(14.0)?;
    ctrls.btn_hide.SetMargin(Thickness {
        Left: 50.0,
        Top: 20.0,
        Right: 80.0,
        Bottom: 20.0,
    })?;
    // ctrls.btn_hide.SetCornerRadius(CornerRadius {
    //     TopLeft: 10.0,
    //     TopRight: 10.0,
    //     BottomLeft: 10.0,
    //     BottomRight: 10.0,
    // })?;
    ctrls.btn_hide.Click(RoutedEventHandler::new(
        enclose! { (cursors, ctrls, stored_hwnd_lock) move |_, _| {
            // let elem: FrameworkElement = sender.as_ref().unwrap().cast()?;
            // let xaml_container: RelativePanel = elem.Parent()?.cast()?;
            // let text_block_resp_1: TextBlock =
            //     find_name_in_panel(&ctrls.container.clone().into(), "text_block_resp_1")?.cast()?;
            let stored_hwnd = *stored_hwnd_lock.read().unwrap();
            let core_window = CoreWindow::GetForCurrentThread()?;
            let core_dispatcher = core_window.Dispatcher()?;
            ctrls.btn_hide.SetIsEnabled(true)?;
            thread::spawn(
                enclose! { (cursors, ctrls, core_dispatcher) move || {
                    // Security flaw: CoreDispatcher::RunAsync may send non-Send closures to another
                    //                thread by invoking DispatchedHandler::Invoke, which is UNSOUND.
                    let handler_before = DispatchedHandler::new(
                        enclose! { (cursors, ctrls) move || {
                            ctrls.container.SetIsHitTestVisible(false)?;
                            set_core_window_cursor(&cursors.no)?;
                            ctrls.prog_ring.SetIsActive(true)?;
                            ctrls.text_resp.SetText("Please wait...")?;
                            Ok(())
                        }},
                    );
                    core_dispatcher.RunAsync(CoreDispatcherPriority::Normal, handler_before).unwrap();
                    let succeeded = hide_window_from_capture(stored_hwnd);
                    let handler_after = DispatchedHandler::new(
                        enclose! { (cursors, ctrls) move || {
                            if succeeded {
                                ctrls.btn_hide.SetIsEnabled(false)?;
                                ctrls.text_resp.SetText("Hide succeeded.")?;
                            } else {
                                ctrls.text_resp.SetText("Hide failed.")?;
                            }
                            ctrls.prog_ring.SetIsActive(false)?;
                            set_core_window_cursor(&cursors.arrow)?;
                            ctrls.container.SetIsHitTestVisible(true)?;
                            Ok(())
                        }},
                    );
                    core_dispatcher.RunAsync(CoreDispatcherPriority::Normal, handler_after).unwrap();
                }}
            );
            Ok(())
        }},
    ))?;
    ctrls.btn_hide.SetIsEnabled(false)?;
    let rectangle_size = 40.0;
    RelativePanel::SetRightOf(&ctrls.rectangle, &ctrls.btn_hide)?;
    RelativePanel::SetAlignVerticalCenterWith(&ctrls.rectangle, &ctrls.btn_hide)?;
    ctrls.rectangle.SetWidth(rectangle_size)?;
    ctrls.rectangle.SetHeight(rectangle_size)?;
    ctrls.rectangle.SetFill(&brushes.white)?;
    ctrls.rectangle.SetStroke(&brushes.black)?;
    ctrls.rectangle.PointerPressed(PointerEventHandler::new(
        enclose! { (ctrls, cursors) move |_, e| {
            let e = e.as_ref().unwrap();
            set_core_window_cursor(&cursors.cross)?;
            ctrls.rectangle.CapturePointer(e.Pointer()?)?;
            e.SetHandled(true)?;
            Ok(())
        }},
    ))?;
    ctrls.rectangle.PointerReleased(PointerEventHandler::new(
        enclose! { (ctrls, cursors, stored_hwnd_lock) move |_, e| {
            let elem: FrameworkElement = ctrls.rectangle.cast()?;
            let e = e.as_ref().unwrap();
            let pointer = e.Pointer()?;
            if is_pointer_captured_by_element(&elem, &pointer) {
                if is_point_in_element(&e.GetCurrentPoint(None)?.Position()?, &elem.clone().into()) {
                    set_core_window_cursor(&cursors.hand)?;
                } else {
                    set_core_window_cursor(&cursors.arrow)?;
                }
                ctrls.rectangle.ReleasePointerCapture(e.Pointer()?)?;

                let hwnd_target = *stored_hwnd_lock.read().unwrap();
                let hidden = is_window_hidden_from_capture(hwnd_target);
                let mut msg = format!("Final hwnd = 0x{:08x}", hwnd_target.0);
                msg += &format!("\nWindow name: {}", get_window_text_as_hstring(hwnd_target));
                msg += &format!("\nWindow class: {}", get_class_name_as_hstring(hwnd_target));
                if hidden {
                    msg += "\n* This windows is already hidden, no action is required.";
                }
                ctrls.text_resp.SetText(msg)?;

                ctrls.btn_hide.SetIsEnabled(!hidden)?;
            }
            Ok(())
        }},
    ))?;
    ctrls.rectangle.PointerEntered(PointerEventHandler::new(
        enclose! { (ctrls, cursors) move |_, e| {
            let elem: FrameworkElement = ctrls.rectangle.cast()?;
            let e = e.as_ref().unwrap();
            let pointer = e.Pointer()?;
            if !is_pointer_captured_by_element(&elem, &pointer) {
                set_core_window_cursor(&cursors.hand)?;
            }
            Ok(())
        }},
    ))?;
    ctrls.rectangle.PointerExited(PointerEventHandler::new(
        enclose! { (ctrls, cursors) move |_, e| {
            let elem: FrameworkElement = ctrls.rectangle.cast()?;
            let e = e.as_ref().unwrap();
            let pointer = e.Pointer()?;
            if !is_pointer_captured_by_element(&elem, &pointer) {
                set_core_window_cursor(&cursors.arrow)?;
            }
            Ok(())
        }},
    ))?;
    ctrls.rectangle.PointerMoved(PointerEventHandler::new(
        enclose! { (ctrls, stored_hwnd_lock) move |_, e| {
            let elem: FrameworkElement = ctrls.rectangle.cast()?;
            let e = e.as_ref().unwrap();
            let pointer = e.Pointer()?;

            if is_pointer_captured_by_element(&elem, &pointer) {
                let cursor_pos = get_cursor_pos();
                let hwnd_target = find_target_window_from_point(&cursor_pos);
                *stored_hwnd_lock.write().unwrap() = hwnd_target;
                // Only lower 32 bits of HWND are valid
                let mut msg = format!("Pointing to hwnd = 0x{:08x}", hwnd_target.0);
                msg += &format!("\nWindow name: {}", get_window_text_as_hstring(hwnd_target));
                msg += &format!("\nWindow class: {}", get_class_name_as_hstring(hwnd_target));
                ctrls.text_resp.SetText(msg)?;
            }

            e.SetHandled(true)?;
            Ok(())
        }},
    ))?;
    ctrls.rectangle_h.SetWidth(rectangle_size - 5.0 * 2.0)?;
    ctrls.rectangle_h.SetHeight(3.0)?;
    RelativePanel::SetAlignVerticalCenterWith(&ctrls.rectangle_h, &ctrls.rectangle)?;
    RelativePanel::SetAlignHorizontalCenterWith(&ctrls.rectangle_h, &ctrls.rectangle)?;
    ctrls.rectangle_h.SetIsHitTestVisible(false)?;
    ctrls.rectangle_h.SetFill(&brushes.black)?;
    ctrls.rectangle_v.SetWidth(3.0)?;
    ctrls.rectangle_v.SetHeight(rectangle_size - 5.0 * 2.0)?;
    RelativePanel::SetAlignVerticalCenterWith(&ctrls.rectangle_v, &ctrls.rectangle)?;
    RelativePanel::SetAlignHorizontalCenterWith(&ctrls.rectangle_v, &ctrls.rectangle)?;
    ctrls.rectangle_v.SetIsHitTestVisible(false)?;
    ctrls.rectangle_v.SetFill(&brushes.black)?;
    ctrls
        .text_resp
        .SetTextWrapping(TextWrapping::WrapWholeWords)?;
    ctrls.text_resp.SetIsTextSelectionEnabled(true)?;
    // ctrls.text_resp.SetName("text_block_resp_1")?;
    ctrls.text_resp.SetText("To begin, drag the cross.")?;
    RelativePanel::SetBelow(&ctrls.text_resp, &ctrls.btn_hide)?;
    ctrls.prog_ring.SetWidth(40.0)?;
    ctrls.prog_ring.SetHeight(40.0)?;
    RelativePanel::SetBelow(&ctrls.prog_ring, &ctrls.text_resp)?;
    ctrls.prog_ring.SetMargin(Thickness {
        Left: 20.0,
        Top: 20.0,
        Right: 0.0,
        Bottom: 0.0,
    })?;
    // ctrls.container.Children()?.Append(text_block_help_1)?;
    // ctrls.container.Children()?.Append(button)?;
    // ctrls.container.Children()?.Append(rectangle)?;
    // ctrls.container.Children()?.Append(rectangle_h)?;
    // ctrls.container.Children()?.Append(rectangle_v)?;
    // ctrls.container.Children()?.Append(text_block_resp_1)?;
    // ctrls.container.Children()?.Append(progress_ring)?;
    // ctrls.container.Children()?.ReplaceAll(
    //     &[Somectrls.text_help.into())]
    // )?;
    let children_collection = ctrls.container.Children()?;
    children_collection.Append(&ctrls.text_help)?;
    children_collection.Append(&ctrls.btn_hide)?;
    children_collection.Append(&ctrls.rectangle)?;
    children_collection.Append(&ctrls.rectangle_h)?;
    children_collection.Append(&ctrls.rectangle_v)?;
    children_collection.Append(&ctrls.text_resp)?;
    children_collection.Append(&ctrls.prog_ring)?;

    desktop_source.SetContent(ctrls.container)?;

    window.set_visible(true);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => {
                *control_flow = ControlFlow::Exit;
                window.set_visible(false);
            }
            _ => (),
        }
    });
}
