//! Paint native controls without depending on private Windows dark-mode APIs.
use std::{
    cell::Cell,
    mem::zeroed,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::{Dwm::*, Gdi::*},
    System::Registry::*,
    UI::{
        Controls::*,
        Input::KeyboardAndMouse::{IsWindowEnabled, SetActiveWindow},
        Shell::*,
        WindowsAndMessaging::*,
    },
};

const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | (g << 8) | (b << 16)
}
#[derive(Clone, Copy)]
pub struct Palette {
    pub background: u32,
    pub panel: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    pub input: u32,
    pub border: u32,
    pub selected: u32,
}
impl Palette {
    pub const fn light() -> Self {
        Self {
            background: rgb(255, 255, 255),
            panel: rgb(242, 247, 252),
            text: rgb(32, 43, 55),
            muted: rgb(80, 91, 103),
            accent: rgb(21, 85, 184),
            input: rgb(255, 255, 255),
            border: rgb(154, 163, 173),
            selected: rgb(211, 231, 248),
        }
    }
    pub const fn dark() -> Self {
        Self {
            background: rgb(22, 27, 34),
            panel: rgb(29, 40, 55),
            text: rgb(232, 237, 245),
            muted: rgb(193, 201, 211),
            accent: rgb(132, 184, 255),
            input: rgb(34, 43, 55),
            border: rgb(89, 103, 120),
            selected: rgb(35, 77, 115),
        }
    }
}
thread_local! { static COLORS: Cell<Palette> = const { Cell::new(Palette::light()) }; }
pub fn colors() -> Palette {
    COLORS.get()
}
pub fn set_dark(dark: bool) {
    COLORS.set(if dark {
        Palette::dark()
    } else {
        Palette::light()
    });
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub unsafe fn redirect_modal(hwnd: HWND, message: u32, w: WPARAM) -> bool {
    if matches!(message, WM_ACTIVATE | WM_SETFOCUS)
        && IsWindowEnabled(hwnd) == 0
        && (message == WM_SETFOCUS || (w & 0xffff) != WA_INACTIVE as usize)
    {
        let popup = GetLastActivePopup(hwnd);
        if popup != hwnd && IsWindowVisible(popup) != 0 {
            SetActiveWindow(popup);
            SetForegroundWindow(popup);
            windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(popup);
        }
        return true;
    }
    false
}

pub fn system_is_dark() -> bool {
    let mut value: u32 = 1;
    let mut size = 4;
    // Read only the user's Windows app-theme preference.
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize").as_ptr(),
            wide("AppsUseLightTheme").as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            &mut value as *mut _ as *mut _,
            &mut size,
        ) == ERROR_SUCCESS
            && value == 0
    }
}
pub unsafe fn title_bar(hwnd: HWND, dark: bool) {
    let enabled = i32::from(dark);
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
        &enabled as *const _ as *const _,
        4,
    );
    // Explicit colors also honor a forced theme that differs from Windows.
    let p = colors();
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_CAPTION_COLOR as u32,
        &p.background as *const _ as *const _,
        4,
    );
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_TEXT_COLOR as u32,
        &p.text as *const _ as *const _,
        4,
    );
}
pub unsafe fn color_control(message: u32, w: WPARAM, l: LPARAM) -> Option<LRESULT> {
    if !matches!(
        message,
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX | WM_CTLCOLORBTN
    ) {
        return None;
    }
    let p = colors();
    let dc = w as HDC;
    let id = GetDlgCtrlID(l as HWND);
    let result = message == WM_CTLCOLORSTATIC && (20..=22).contains(&id);
    let background = if result {
        p.panel
    } else if matches!(message, WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX) {
        p.input
    } else {
        p.background
    };
    SetBkMode(dc, OPAQUE as i32);
    SetBkColor(dc, background);
    SetTextColor(dc, if id == 21 && result { p.accent } else { p.text });
    SetDCBrushColor(dc, background);
    Some(GetStockObject(DC_BRUSH) as isize)
}
unsafe fn fill(dc: HDC, rect: &RECT, color: u32) {
    SetDCBrushColor(dc, color);
    FillRect(dc, rect, GetStockObject(DC_BRUSH));
}
unsafe fn frame(dc: HDC, rect: &RECT, color: u32) {
    SetDCBrushColor(dc, color);
    FrameRect(dc, rect, GetStockObject(DC_BRUSH));
}
unsafe fn window_text(hwnd: HWND) -> Vec<u16> {
    let mut value = vec![0u16; GetWindowTextLengthW(hwnd).max(0) as usize + 1];
    GetWindowTextW(hwnd, value.as_mut_ptr(), value.len() as i32);
    value
}
pub unsafe fn button_custom_draw(l: LPARAM) -> Option<LRESULT> {
    if l == 0 {
        return None;
    }
    let header = &*(l as *const NMHDR);
    if header.code != NM_CUSTOMDRAW {
        return None;
    }
    let id = GetDlgCtrlID(header.hwndFrom);
    if !matches!(id, 10..=12 | 24..=28) {
        return None;
    }
    let draw = &*(l as *const NMCUSTOMDRAW);
    if draw.dwDrawStage != CDDS_PREPAINT {
        return Some(CDRF_DODEFAULT as isize);
    }
    let p = colors();
    let selected = SendMessageW(header.hwndFrom, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
    let pressed = draw.uItemState & CDIS_SELECTED != 0;
    fill(
        draw.hdc,
        &draw.rc,
        if selected || pressed {
            p.selected
        } else {
            p.input
        },
    );
    frame(
        draw.hdc,
        &draw.rc,
        if selected || draw.uItemState & CDIS_HOT != 0 {
            p.accent
        } else {
            p.border
        },
    );
    SetBkMode(draw.hdc, TRANSPARENT as i32);
    SetTextColor(
        draw.hdc,
        if draw.uItemState & CDIS_DISABLED != 0 {
            p.muted
        } else {
            p.text
        },
    );
    let mut rect = draw.rc;
    DrawTextW(
        draw.hdc,
        window_text(header.hwndFrom).as_ptr(),
        -1,
        &mut rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );
    if draw.uItemState & CDIS_FOCUS != 0 {
        InflateRect(&mut rect, -3, -3);
        DrawFocusRect(draw.hdc, &rect);
    }
    Some(CDRF_SKIPDEFAULT as isize)
}

pub unsafe fn install_combo(hwnd: HWND) {
    SetWindowSubclass(hwnd, Some(combo_proc), 1, 0);
}
pub unsafe fn install_edit(hwnd: HWND) {
    SetWindowSubclass(hwnd, Some(edit_proc), 2, 0);
}
unsafe extern "system" fn edit_proc(
    hwnd: HWND,
    message: u32,
    w: WPARAM,
    l: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    match message {
        WM_NCPAINT => {
            let result = DefSubclassProc(hwnd, message, w, l);
            let dc = GetWindowDC(hwnd);
            if !dc.is_null() {
                let mut bounds = zeroed();
                GetWindowRect(hwnd, &mut bounds);
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: bounds.right - bounds.left,
                    bottom: bounds.bottom - bounds.top,
                };
                let p = colors();
                let focused = windows_sys::Win32::UI::Input::KeyboardAndMouse::GetFocus() == hwnd;
                let edge = windows_sys::Win32::UI::HiDpi::GetSystemMetricsForDpi(
                    SM_CXEDGE,
                    windows_sys::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(96),
                );
                for inset in 0..edge {
                    frame(
                        dc,
                        &rect,
                        if inset == 0 {
                            if focused {
                                p.accent
                            } else {
                                p.border
                            }
                        } else {
                            p.input
                        },
                    );
                    InflateRect(&mut rect, -1, -1);
                }
                ReleaseDC(hwnd, dc);
            }
            result
        }
        WM_SETFOCUS | WM_KILLFOCUS => {
            let result = DefSubclassProc(hwnd, message, w, l);
            RedrawWindow(hwnd, null(), null_mut(), RDW_FRAME | RDW_INVALIDATE);
            result
        }
        WM_NCDESTROY => {
            RemoveWindowSubclass(hwnd, Some(edit_proc), 2);
            DefSubclassProc(hwnd, message, w, l)
        }
        _ => DefSubclassProc(hwnd, message, w, l),
    }
}
unsafe fn paint_combo(hwnd: HWND, dc: HDC) {
    let p = colors();
    let mut rect = zeroed();
    GetClientRect(hwnd, &mut rect);
    let dpi = windows_sys::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(96) as i32;
    let scale = |n| n * dpi / 96;
    fill(dc, &rect, p.input);
    frame(
        dc,
        &rect,
        if windows_sys::Win32::UI::Input::KeyboardAndMouse::GetFocus() == hwnd {
            p.accent
        } else {
            p.border
        },
    );
    let font = SendMessageW(hwnd, WM_GETFONT, 0, 0) as HGDIOBJ;
    let old = SelectObject(dc, font);
    SetBkMode(dc, TRANSPARENT as i32);
    SetTextColor(dc, p.text);
    let mut text_rect = rect;
    text_rect.left += scale(7);
    text_rect.right -= scale(26);
    DrawTextW(
        dc,
        window_text(hwnd).as_ptr(),
        -1,
        &mut text_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    SelectObject(dc, old);
    let cx = rect.right - scale(14);
    let cy = rect.bottom / 2;
    let pen = CreatePen(PS_SOLID, 1, p.muted);
    let old = SelectObject(dc, pen);
    MoveToEx(dc, cx - scale(3), cy - scale(1), null_mut());
    LineTo(dc, cx, cy + scale(2));
    LineTo(dc, cx + scale(4), cy - scale(2));
    SelectObject(dc, old);
    DeleteObject(pen);
}
unsafe extern "system" fn combo_proc(
    hwnd: HWND,
    message: u32,
    w: WPARAM,
    l: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    match message {
        WM_PAINT => {
            let mut ps = zeroed();
            let dc = BeginPaint(hwnd, &mut ps);
            paint_combo(hwnd, dc);
            EndPaint(hwnd, &ps);
            0
        }
        WM_PRINTCLIENT => {
            paint_combo(hwnd, w as HDC);
            0
        }
        WM_ERASEBKGND => 1,
        WM_NCDESTROY => {
            RemoveWindowSubclass(hwnd, Some(combo_proc), 1);
            DefSubclassProc(hwnd, message, w, l)
        }
        WM_SETFOCUS | WM_KILLFOCUS | CB_SETCURSEL | WM_ENABLE | CB_SHOWDROPDOWN => {
            let result = DefSubclassProc(hwnd, message, w, l);
            InvalidateRect(hwnd, null(), 0);
            result
        }
        _ => DefSubclassProc(hwnd, message, w, l),
    }
}
pub unsafe fn measure_combo(hwnd: HWND, l: LPARAM) -> bool {
    let item = &mut *(l as *mut MEASUREITEMSTRUCT);
    if item.CtlType != ODT_COMBOBOX {
        return false;
    }
    item.itemHeight = 24 * windows_sys::Win32::UI::HiDpi::GetDpiForWindow(hwnd).max(96) / 96;
    true
}
pub unsafe fn draw_combo(l: LPARAM) -> bool {
    let item = &*(l as *const DRAWITEMSTRUCT);
    if item.CtlType != ODT_COMBOBOX {
        return false;
    }
    let p = colors();
    let selected = item.itemState & ODS_SELECTED != 0;
    fill(
        item.hDC,
        &item.rcItem,
        if selected { p.selected } else { p.input },
    );
    let mut value = if item.itemID == u32::MAX {
        window_text(item.hwndItem)
    } else {
        let len = SendMessageW(item.hwndItem, CB_GETLBTEXTLEN, item.itemID as usize, 0);
        if !(0..4096).contains(&len) {
            return true;
        }
        let mut value = vec![0; len as usize + 1];
        SendMessageW(
            item.hwndItem,
            CB_GETLBTEXT,
            item.itemID as usize,
            value.as_mut_ptr() as isize,
        );
        value
    };
    let old = SelectObject(
        item.hDC,
        SendMessageW(item.hwndItem, WM_GETFONT, 0, 0) as HGDIOBJ,
    );
    SetTextColor(item.hDC, p.text);
    SetBkMode(item.hDC, TRANSPARENT as i32);
    let mut rect = item.rcItem;
    rect.left += 6;
    DrawTextW(
        item.hDC,
        value.as_mut_ptr(),
        -1,
        &mut rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    SelectObject(item.hDC, old);
    true
}
