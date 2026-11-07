//! Native user-substance editor. Save commits core and locale fields together.
use crate::{
    calc, i18n,
    settings::Language,
    substances::Substance,
    theme,
    user_db::{self, UserRecord, UserStore},
};
use std::{
    cell::RefCell,
    mem::zeroed,
    path::Path,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::*,
    UI::{Controls::*, HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
struct Editor<'a> {
    hwnd: HWND,
    dpi: u32,
    font: HFONT,
    language: Language,
    path: &'a Path,
    record: UserRecord,
    saved: Option<i64>,
    done: bool,
}
impl Editor<'_> {
    fn s(&self, n: i32) -> i32 {
        (n as i64 * self.dpi as i64 / 96) as i32
    }
    unsafe fn get(&self, id: i32) -> HWND {
        GetDlgItem(self.hwnd, id)
    }
    unsafe fn text(&self, id: i32) -> String {
        let h = self.get(id);
        let mut t = vec![0u16; GetWindowTextLengthW(h).max(0) as usize + 1];
        let len = GetWindowTextW(h, t.as_mut_ptr(), t.len() as i32);
        String::from_utf16_lossy(&t[..len.max(0) as usize])
    }
    #[allow(clippy::too_many_arguments)]
    unsafe fn control(
        &self,
        class: &str,
        text: &str,
        id: i32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        style: u32,
    ) -> HWND {
        let hwnd = CreateWindowExW(
            if class == "EDIT" { WS_EX_CLIENTEDGE } else { 0 },
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            self.s(x),
            self.s(y),
            self.s(w),
            self.s(h),
            self.hwnd,
            id as usize as HMENU,
            GetModuleHandleW(null()),
            null(),
        );
        SendMessageW(hwnd, WM_SETFONT, self.font as usize, 1);
        if class == "EDIT" {
            theme::install_edit(hwnd);
            SendMessageW(
                hwnd,
                EM_SETLIMITTEXT,
                if id == 102 || id == 107 { 2000 } else { 200 },
                0,
            );
        }
        hwnd
    }
    unsafe fn row(&self, label: &str, id: i32, value: &str, y: i32) {
        self.control("STATIC", label, 0, 20, y + 4, 174, 24, 0);
        self.control(
            "EDIT",
            value,
            id,
            200,
            y,
            460,
            28,
            WS_TABSTOP | ES_AUTOHSCROLL as u32,
        );
    }
    unsafe fn create(&self) {
        let lang = self.language;
        let r = &self.record;
        self.row(
            lang.choose("Name (EN)", "Название (EN)"),
            101,
            &r.name_en,
            20,
        );
        self.row(
            lang.choose("Synonyms (EN)", "Синонимы (EN)"),
            102,
            &r.synonyms_en,
            60,
        );
        let extra = if lang.code() != "en" { 80 } else { 0 };
        if extra > 0 {
            self.row(
                &format!(
                    "{} ({})",
                    lang.choose("Name", "Название"),
                    lang.code().to_uppercase()
                ),
                106,
                &r.local_name,
                100,
            );
            self.row(
                &format!(
                    "{} ({})",
                    lang.choose("Synonyms", "Синонимы"),
                    lang.code().to_uppercase()
                ),
                107,
                &r.local_synonyms,
                140,
            );
        }
        self.row(
            lang.choose("Formula (optional)", "Формула (если есть)"),
            103,
            &r.formula,
            100 + extra,
        );
        self.row(
            lang.choose("Molar mass, g/mol", "Молярная масса, g/mol"),
            104,
            &if r.molar_mass > 0.0 {
                i18n::numbers(lang, &r.molar_mass.to_string()).into_owned()
            } else {
                String::new()
            },
            140 + extra,
        );
        self.row(
            lang.choose("CAS (optional)", "CAS (необязательно)"),
            105,
            &r.cas,
            180 + extra,
        );
        for id in [102, 107] {
            let hwnd = self.get(id);
            if !hwnd.is_null() {
                SendMessageW(
                    hwnd,
                    EM_SETCUEBANNER,
                    0,
                    wide(lang.choose("Separate with semicolons", "Разделяйте точкой с запятой"))
                        .as_ptr() as isize,
                );
            }
        }
        self.control(
            "BUTTON",
            lang.choose("&Save", "&Сохранить"),
            24,
            408,
            230 + extra,
            120,
            32,
            WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
        );
        self.control(
            "BUTTON",
            lang.choose("Cancel", "Отмена"),
            25,
            540,
            230 + extra,
            120,
            32,
            WS_TABSTOP,
        );
    }
    unsafe fn save(&mut self) {
        let parsed = calc::parse(&self.text(104), "Molar mass");
        let mass = match parsed {
            Ok(Some(v)) => v,
            Ok(None) => {
                self.error("Enter molar mass.");
                return;
            }
            Err(e) => {
                self.error(&i18n::error(self.language, &e.0));
                return;
            }
        };
        let mut draft = self.record.clone();
        draft.name_en = self.text(101);
        draft.synonyms_en = self.text(102);
        draft.formula = self.text(103);
        draft.molar_mass = mass;
        draft.cas = self.text(105);
        draft.locale = self.language.code().into();
        if draft.locale != "en" {
            draft.local_name = self.text(106);
            draft.local_synonyms = self.text(107);
        }
        let result =
            user_db::validate(&draft).and_then(|_| UserStore::open(self.path)?.save(&draft));
        match result {
            Ok(id) => {
                self.saved = Some(id);
                self.done = true;
            }
            Err(error) => self.error(&error),
        }
    }
    unsafe fn error(&self, error: &str) {
        MessageBoxW(
            self.hwnd,
            wide(i18n::text(self.language, error)).as_ptr(),
            wide(
                self.language
                    .choose("Could not save substance", "Не удалось сохранить вещество"),
            )
            .as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
unsafe extern "system" fn proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if theme::redirect_modal(hwnd, message, w) {
        return 0;
    }
    if message == WM_SETFOCUS {
        SetFocus(GetDlgItem(hwnd, 101));
        return 0;
    }
    if message == WM_NCCREATE {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (*(l as *const CREATESTRUCTW)).lpCreateParams as isize,
        );
    }
    if message == WM_NCDESTROY {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        return DefWindowProcW(hwnd, message, w, l);
    }
    if let Some(result) = theme::color_control(message, w, l) {
        return result;
    }
    if message == WM_NOTIFY {
        if let Some(result) = theme::button_custom_draw(l) {
            return result;
        }
    }
    if !matches!(message, WM_COMMAND | WM_CLOSE | WM_PAINT | WM_ERASEBKGND) {
        return DefWindowProcW(hwnd, message, w, l);
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<Editor<'_>>;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, message, w, l);
    }
    let Ok(mut editor) = (*ptr).try_borrow_mut() else {
        return DefWindowProcW(hwnd, message, w, l);
    };
    match message {
        WM_CLOSE => {
            editor.done = true;
            0
        }
        WM_COMMAND => {
            match (w & 0xffff, (w >> 16) as u16) {
                (24, 0) => editor.save(),
                (25, 0) => editor.done = true,
                _ => {}
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut ps = zeroed();
            let dc = BeginPaint(hwnd, &mut ps);
            let mut rect = zeroed();
            GetClientRect(hwnd, &mut rect);
            SetDCBrushColor(dc, theme::colors().background);
            FillRect(dc, &rect, GetStockObject(DC_BRUSH));
            EndPaint(hwnd, &ps);
            0
        }
        _ => 0,
    }
}
pub unsafe fn edit(
    owner: HWND,
    language: Language,
    dark: bool,
    path: &Path,
    template: Option<&Substance>,
) -> Option<i64> {
    let record = if let Some(entry) = template {
        if let Some(id) = entry.user_id {
            match UserStore::open(path).and_then(|s| s.get(id, language.code())) {
                Ok(r) => r,
                Err(e) => {
                    MessageBoxW(
                        owner,
                        wide(&e).as_ptr(),
                        wide("Solcalc").as_ptr(),
                        MB_OK | MB_ICONERROR,
                    );
                    return None;
                }
            }
        } else {
            entry.user_record(language.code())
        }
    } else {
        UserRecord {
            locale: language.code().into(),
            ..Default::default()
        }
    };
    let instance = GetModuleHandleW(null());
    let class = wide("Solcalc3.UserSubstance");
    let wc = WNDCLASSW {
        lpfnWndProc: Some(proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        lpszClassName: class.as_ptr(),
        ..zeroed()
    };
    RegisterClassW(&wc);
    let dpi = GetDpiForWindow(owner).max(96);
    let s = |n: i32| (n as i64 * dpi as i64 / 96) as i32;
    let font = CreateFontW(
        -s(15),
        0,
        0,
        0,
        400,
        0,
        0,
        0,
        DEFAULT_CHARSET as u32,
        OUT_DEFAULT_PRECIS as u32,
        CLIP_DEFAULT_PRECIS as u32,
        CLEARTYPE_QUALITY as u32,
        DEFAULT_PITCH as u32,
        wide("Segoe UI").as_ptr(),
    );
    let state = Box::new(RefCell::new(Editor {
        hwnd: null_mut(),
        dpi,
        font,
        language,
        path,
        record,
        saved: None,
        done: false,
    }));
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: s(680),
        bottom: s(if language.code() == "en" { 282 } else { 362 }),
    };
    let style = WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_CLIPCHILDREN;
    let ex = WS_EX_DLGMODALFRAME | WS_EX_CONTROLPARENT;
    AdjustWindowRectExForDpi(&mut rect, style, 0, ex, dpi);
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let mut parent = zeroed();
    GetWindowRect(owner, &mut parent);
    let mut monitor: MONITORINFO = zeroed();
    monitor.cbSize = size_of::<MONITORINFO>() as u32;
    GetMonitorInfoW(
        MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST),
        &mut monitor,
    );
    let x = (parent.left + (parent.right - parent.left - width) / 2)
        .max(monitor.rcWork.left)
        .min((monitor.rcWork.right - width).max(monitor.rcWork.left));
    let y = (parent.top + (parent.bottom - parent.top - height) / 2)
        .max(monitor.rcWork.top)
        .min((monitor.rcWork.bottom - height).max(monitor.rcWork.top));
    let hwnd = CreateWindowExW(
        ex,
        class.as_ptr(),
        wide(language.choose("User substance", "Вещество пользователя")).as_ptr(),
        style,
        x,
        y,
        width,
        height,
        owner,
        null_mut(),
        instance,
        state.as_ref() as *const RefCell<Editor<'_>> as *const _,
    );
    if hwnd.is_null() {
        DeleteObject(font);
        return None;
    }
    {
        let mut e = state.borrow_mut();
        e.hwnd = hwnd;
        e.create();
    }
    theme::title_bar(hwnd, dark);
    EnableWindow(owner, 0);
    ShowWindow(hwnd, SW_SHOW);
    SetFocus(GetDlgItem(hwnd, 101));
    let mut message: MSG = zeroed();
    while !state.borrow().done {
        let status = GetMessageW(&mut message, null_mut(), 0, 0);
        if status <= 0 {
            if status == 0 {
                PostQuitMessage(message.wParam as i32);
            }
            break;
        }
        if message.message == WM_KEYDOWN {
            match message.wParam as u16 {
                VK_ESCAPE => {
                    state.borrow_mut().done = true;
                    continue;
                }
                VK_RETURN => {
                    if GetDlgCtrlID(GetFocus()) == 25 {
                        state.borrow_mut().done = true;
                    } else {
                        state.borrow_mut().save();
                    }
                    continue;
                }
                _ => {}
            }
        }
        if IsDialogMessageW(hwnd, &message) == 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    let result = state.borrow().saved;
    EnableWindow(owner, 1);
    DestroyWindow(hwnd);
    SetActiveWindow(owner);
    DeleteObject(font);
    result
}
