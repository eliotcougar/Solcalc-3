//! Native, owned substance picker with incremental offline search.
use crate::{
    calc, i18n, settings::Language, substance_editor, substances::Catalog, theme, user_db,
};
use std::{
    cell::RefCell,
    mem::zeroed,
    path::PathBuf,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{LibraryLoader::*, SystemServices::SS_NOPREFIX},
    UI::{Controls::*, HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

const FILTER: u32 = WM_APP + 20;
const DETAILS: u32 = WM_APP + 21;
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
struct Picker {
    hwnd: HWND,
    language: Language,
    dpi: u32,
    font: HFONT,
    hits: Vec<usize>,
    result: Option<usize>,
    done: bool,
    catalog: Catalog,
    user_path: Option<PathBuf>,
    dark: bool,
    last_query: String,
}
pub struct Choice {
    pub catalog: Catalog,
    pub selected: Option<String>,
}
impl Picker {
    fn s(&self, n: i32) -> i32 {
        (n as i64 * self.dpi as i64 / 96) as i32
    }
    unsafe fn get(&self, id: i32) -> HWND {
        GetDlgItem(self.hwnd, id)
    }
    unsafe fn label(&self, id: i32, text: &str) {
        SetWindowTextW(self.get(id), wide(text).as_ptr());
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
    ) {
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
        }
    }
    unsafe fn create(&mut self, initial: Option<usize>) {
        let lang = self.language;
        self.control(
            "STATIC",
            lang.choose("&Search", "&Поиск"),
            0,
            20,
            18,
            75,
            24,
            0,
        );
        self.control(
            "EDIT",
            "",
            1,
            100,
            14,
            640,
            30,
            WS_TABSTOP | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(self.get(1), EM_SETLIMITTEXT, 200, 0);
        SendMessageW(
            self.get(1),
            EM_SETCUEBANNER,
            0,
            wide(lang.choose(
                "Name, synonym, formula, or CAS",
                "Название, синоним, формула или CAS",
            ))
            .as_ptr() as isize,
        );
        self.control(
            "LISTBOX",
            "",
            2,
            20,
            60,
            720,
            314,
            WS_TABSTOP
                | WS_VSCROLL
                | WS_BORDER
                | LBS_NOTIFY as u32
                | LBS_OWNERDRAWFIXED as u32
                | LBS_HASSTRINGS as u32
                | LBS_NOINTEGRALHEIGHT as u32,
        );
        SendMessageW(self.get(2), LB_SETITEMHEIGHT, 0, self.s(54) as isize);
        self.control("STATIC", "", 3, 20, 394, 720, 74, SS_NOPREFIX);
        self.control("STATIC", "", 4, 20, 374, 720, 22, 0);
        self.control(
            "BUTTON",
            lang.choose("&Add…", "&Добавить…"),
            26,
            20,
            480,
            110,
            32,
            WS_TABSTOP,
        );
        self.control(
            "BUTTON",
            lang.choose("&Edit / customize…", "&Изменить…"),
            27,
            142,
            480,
            170,
            32,
            WS_TABSTOP,
        );
        EnableWindow(self.get(26), i32::from(self.user_path.is_some()));
        self.control(
            "BUTTON",
            lang.choose("&Delete", "&Удалить"),
            28,
            324,
            480,
            116,
            32,
            WS_TABSTOP,
        );
        self.control(
            "BUTTON",
            lang.choose("&Use substance", "&Выбрать"),
            24,
            452,
            480,
            160,
            32,
            WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
        );
        self.control(
            "BUTTON",
            lang.choose("Cancel", "Отмена"),
            25,
            624,
            480,
            116,
            32,
            WS_TABSTOP,
        );
        self.filter(initial);
    }
    unsafe fn filter(&mut self, preferred: Option<usize>) {
        let edit = self.get(1);
        let mut query = vec![0u16; GetWindowTextLengthW(edit).max(0) as usize + 1];
        let len = GetWindowTextW(edit, query.as_mut_ptr(), query.len() as i32);
        let query = String::from_utf16_lossy(&query[..len.max(0) as usize]);
        let preferred = preferred.or_else(|| {
            if query == self.last_query {
                self.selected()
            } else {
                None
            }
        });
        self.last_query = query.clone();
        self.hits = self.catalog.search(&query, self.language);
        let list = self.get(2);
        SendMessageW(list, WM_SETREDRAW, 0, 0);
        SendMessageW(list, LB_RESETCONTENT, 0, 0);
        for i in &self.hits {
            let entry = &self.catalog.entries[*i];
            // Native list text remains readable to accessibility clients.
            let value = format!(
                "{} — {}{} g/mol{}{}",
                entry.name(self.language),
                if entry.formula.is_empty() {
                    String::new()
                } else {
                    format!("{} — ", entry.formula_display())
                },
                i18n::numbers(self.language, &calc::number(entry.molar_mass)),
                if entry.cas.is_empty() {
                    String::new()
                } else {
                    format!(" — CAS {}", entry.cas)
                },
                if entry.user_id.is_some() {
                    self.language.choose(" — User", " — Пользователь")
                } else {
                    ""
                }
            );
            SendMessageW(list, LB_ADDSTRING, 0, wide(&value).as_ptr() as isize);
        }
        let index = preferred
            .and_then(|id| self.hits.iter().position(|i| *i == id))
            .unwrap_or(0);
        if !self.hits.is_empty() {
            SendMessageW(list, LB_SETCURSEL, index, 0);
        }
        SendMessageW(list, WM_SETREDRAW, 1, 0);
        InvalidateRect(list, null(), 1);
        self.label(
            4,
            &if self.hits.is_empty() {
                self.language
                    .choose("No substances found", "Ничего не найдено")
                    .to_owned()
            } else {
                format!(
                    "{}: {}",
                    self.language.choose("Found", "Найдено"),
                    self.hits.len()
                )
            },
        );
        EnableWindow(self.get(24), i32::from(!self.hits.is_empty()));
        EnableWindow(
            self.get(27),
            i32::from(!self.hits.is_empty() && self.user_path.is_some()),
        );
        self.details();
    }
    unsafe fn selected(&self) -> Option<usize> {
        let selected = SendMessageW(self.get(2), LB_GETCURSEL, 0, 0);
        (selected >= 0)
            .then(|| self.hits.get(selected as usize).copied())
            .flatten()
    }
    unsafe fn details(&self) {
        let can_delete = self.user_path.is_some()
            && self
                .selected()
                .is_some_and(|i| self.catalog.entries[i].user_id.is_some());
        EnableWindow(self.get(28), i32::from(can_delete));
        let text = self
            .selected()
            .map(|i| {
                let entry = &self.catalog.entries[i];
                let source = if entry.user_id.is_some() {
                    self.language.choose("User database", "База пользователя")
                } else {
                    self.language.choose("Bundled database", "База приложения")
                };
                let mut lines = vec![if entry.cas.is_empty() {
                    source.to_owned()
                } else {
                    format!("CAS: {} · {source}", entry.cas)
                }];
                if self.language.code() != "en" && !entry.name_en.is_empty() {
                    lines.push(entry.name_en.clone());
                }
                let synonyms = entry.synonyms();
                if !synonyms.is_empty() {
                    lines.push(format!(
                        "{}: {}",
                        self.language.choose("Synonyms", "Синонимы"),
                        synonyms.replace(';', "; ")
                    ));
                }
                lines.join("\n")
            })
            .unwrap_or_default();
        self.label(3, &text);
    }
    unsafe fn accept(&mut self) {
        // Flush any queued incremental filtering before accepting Enter.
        // Preserve the selected row if it still matches the current query.
        self.filter(self.selected());
        if let Some(i) = self.selected() {
            self.result = Some(i);
            self.done = true;
        }
    }
    unsafe fn draw(&self, draw: &DRAWITEMSTRUCT) {
        let p = theme::colors();
        let selected = draw.itemState & ODS_SELECTED != 0;
        let rect = draw.rcItem;
        SetDCBrushColor(draw.hDC, if selected { p.selected } else { p.input });
        FillRect(draw.hDC, &rect, GetStockObject(DC_BRUSH));
        let Some(id) = self.hits.get(draw.itemID as usize) else {
            return;
        };
        let entry = &self.catalog.entries[*id];
        SelectObject(draw.hDC, self.font);
        SetBkMode(draw.hDC, TRANSPARENT as i32);
        SetTextColor(draw.hDC, p.text);
        let mut line = RECT {
            left: rect.left + self.s(10),
            top: rect.top + self.s(4),
            right: rect.right - self.s(10),
            bottom: rect.top + self.s(28),
        };
        DrawTextW(
            draw.hDC,
            wide(entry.name(self.language)).as_ptr(),
            -1,
            &mut line,
            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        line.top = rect.top + self.s(28);
        line.bottom = rect.bottom - self.s(3);
        SetTextColor(draw.hDC, p.muted);
        let detail = format!(
            "{}{} g/mol{}{}",
            if entry.formula.is_empty() {
                String::new()
            } else {
                format!("{}     ", entry.formula_display())
            },
            i18n::numbers(self.language, &calc::number(entry.molar_mass)),
            if entry.cas.is_empty() {
                String::new()
            } else {
                format!("     CAS {}", entry.cas)
            },
            if entry.user_id.is_some() {
                self.language.choose("     User", "     Пользователь")
            } else {
                ""
            }
        );
        DrawTextW(
            draw.hDC,
            wide(&detail).as_ptr(),
            -1,
            &mut line,
            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        if draw.itemState & ODS_FOCUS != 0 {
            DrawFocusRect(draw.hDC, &rect);
        }
    }
}

unsafe extern "system" fn proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if theme::redirect_modal(hwnd, message, w) {
        return 0;
    }
    if message == WM_SETFOCUS {
        SetFocus(GetDlgItem(hwnd, 1));
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
    if message == WM_MEASUREITEM {
        let measure = &mut *(l as *mut MEASUREITEMSTRUCT);
        if measure.CtlID == 2 {
            measure.itemHeight = GetDpiForWindow(hwnd).max(96) * 54 / 96;
            return 1;
        }
    }
    if message == WM_COMMAND && (w & 0xffff) == 1 && ((w >> 16) as u16) == EN_CHANGE as u16 {
        PostMessageW(hwnd, FILTER, 0, 0);
        return 0;
    }
    if message == WM_COMMAND && (w & 0xffff) == 2 && ((w >> 16) as u16) == LBN_SELCHANGE as u16 {
        PostMessageW(hwnd, DETAILS, 0, 0);
        return 0;
    }
    if !matches!(
        message,
        WM_CLOSE | WM_COMMAND | FILTER | DETAILS | WM_DRAWITEM | WM_ERASEBKGND | WM_PAINT
    ) {
        return DefWindowProcW(hwnd, message, w, l);
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<Picker>;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, message, w, l);
    }
    if message == WM_COMMAND && w & 0xffff == 28 && (w >> 16) as u16 == BN_CLICKED as u16 {
        // Release the picker borrow while the confirmation owns its modal loop.
        let (language, path, entry) = {
            let p = (*ptr).borrow();
            (
                p.language,
                p.user_path.clone(),
                p.selected().map(|i| p.catalog.entries[i].clone()),
            )
        };
        if let (Some(path), Some(entry)) = (path, entry) {
            if let (Some(id), Some(revision)) = (entry.user_id, entry.user_revision) {
                let question = if entry.id.starts_with("bundled:") {
                    language.choose("Delete the user customization of this substance? The bundled substance will remain available.","Удалить пользовательские изменения этого вещества? Вещество из базы приложения останется доступно.")
                } else {
                    language.choose("Delete this substance from the user database, including all its localized names and synonyms?","Удалить это вещество из базы пользователя, включая все его названия и синонимы на всех языках?")
                };
                let answer = MessageBoxExW(
                    hwnd,
                    wide(&format!("{}\n\n{}", entry.name(language), question)).as_ptr(),
                    wide(
                        language.choose("Delete user substance", "Удаление вещества пользователя"),
                    )
                    .as_ptr(),
                    MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
                    if language == Language::Ru { 1049 } else { 1033 },
                );
                if answer == IDYES {
                    let result = user_db::UserStore::open(&path)
                        .and_then(|mut db| db.delete(id, revision))
                        .and_then(|_| Catalog::load(language.code(), &path));
                    match result {
                        Ok(catalog) => {
                            let preferred = catalog.entries.iter().position(|e| e.id == entry.id);
                            let mut p = (*ptr).borrow_mut();
                            p.catalog = catalog;
                            p.hits.clear();
                            p.filter(preferred);
                            SetFocus(p.get(2));
                        }
                        Err(error) => {
                            MessageBoxW(
                                hwnd,
                                wide(i18n::text(language, &error)).as_ptr(),
                                wide(language.choose(
                                    "Could not delete substance",
                                    "Не удалось удалить вещество",
                                ))
                                .as_ptr(),
                                MB_OK | MB_ICONERROR,
                            );
                        }
                    }
                }
            }
        }
        return 0;
    }
    if message == WM_COMMAND
        && matches!(w & 0xffff, 26 | 27)
        && (w >> 16) as u16 == BN_CLICKED as u16
    {
        let (language, dark, path, template) = {
            let p = (*ptr).borrow();
            (
                p.language,
                p.dark,
                p.user_path.clone(),
                if w & 0xffff == 27 {
                    p.selected().map(|i| p.catalog.entries[i].clone())
                } else {
                    None
                },
            )
        };
        if let Some(path) = path {
            if let Some(saved) =
                substance_editor::edit(hwnd, language, dark, &path, template.as_ref())
            {
                match Catalog::load(language.code(), &path) {
                    Ok(catalog) => {
                        let preferred = catalog
                            .entries
                            .iter()
                            .position(|e| e.user_id == Some(saved));
                        let mut p = (*ptr).borrow_mut();
                        p.catalog = catalog;
                        SetWindowTextW(p.get(1), wide("").as_ptr());
                        p.filter(preferred);
                        SetFocus(p.get(2));
                    }
                    Err(e) => {
                        MessageBoxW(
                            hwnd,
                            wide(&e).as_ptr(),
                            wide("Solcalc").as_ptr(),
                            MB_OK | MB_ICONERROR,
                        );
                    }
                }
            }
        }
        return 0;
    }
    let Ok(mut picker) = (*ptr).try_borrow_mut() else {
        return DefWindowProcW(hwnd, message, w, l);
    };
    match message {
        WM_CLOSE => {
            picker.done = true;
            0
        }
        WM_COMMAND => {
            match (w & 0xffff, (w >> 16) as u16) {
                (24, 0) | (2, 2) => picker.accept(), // BN_CLICKED / LBN_DBLCLK
                (25, 0) => picker.done = true,
                _ => {}
            }
            0
        }
        FILTER => {
            picker.filter(None);
            0
        }
        DETAILS => {
            picker.details();
            0
        }
        WM_DRAWITEM => {
            picker.draw(&*(l as *const DRAWITEMSTRUCT));
            1
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

/// Owns its message loop; caller must release all parent UI state borrows.
pub unsafe fn choose(
    owner: HWND,
    language: Language,
    dark: bool,
    initial: Option<String>,
) -> Choice {
    let user_path = user_db::path();
    let loaded = user_path
        .as_ref()
        .ok_or_else(|| "LOCALAPPDATA is unavailable".to_owned())
        .and_then(|p| Catalog::load(language.code(), p));
    let catalog = match loaded {
        Ok(c) => c,
        Err(error) => {
            MessageBoxW(owner,wide(&format!("{}\n{}",language.choose("Could not load the user database. The file was not replaced. Bundled substances are available.","Не удалось загрузить базу пользователя. Файл не заменён. Доступны вещества из базы приложения."),i18n::text(language,&error))).as_ptr(),wide("Solcalc").as_ptr(),MB_OK|MB_ICONERROR);
            Catalog::bundled(language.code()).expect("Invalid bundled catalog")
        }
    };
    let initial = initial
        .as_ref()
        .and_then(|id| catalog.entries.iter().position(|e| e.id == *id));
    let instance = GetModuleHandleW(null());
    let class = wide("Solcalc3.SubstancePicker");
    let wc = WNDCLASSW {
        lpfnWndProc: Some(proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        lpszClassName: class.as_ptr(),
        ..zeroed()
    };
    RegisterClassW(&wc); // Already registered when the picker is reopened.
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
    let state = Box::new(RefCell::new(Picker {
        hwnd: null_mut(),
        language,
        dpi,
        font,
        hits: vec![],
        result: None,
        done: false,
        catalog,
        user_path,
        dark,
        last_query: String::new(),
    }));
    let mut parent = zeroed();
    GetWindowRect(owner, &mut parent);
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: s(760),
        bottom: s(530),
    };
    let style = WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_CLIPCHILDREN;
    AdjustWindowRectExForDpi(
        &mut rect,
        style,
        0,
        WS_EX_DLGMODALFRAME | WS_EX_CONTROLPARENT,
        dpi,
    );
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
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
        WS_EX_DLGMODALFRAME | WS_EX_CONTROLPARENT,
        class.as_ptr(),
        wide(language.choose("Substances", "Вещества")).as_ptr(),
        style,
        x,
        y,
        width,
        height,
        owner,
        null_mut(),
        instance,
        state.as_ref() as *const RefCell<Picker> as *const _,
    );
    if hwnd.is_null() {
        DeleteObject(font);
        return Choice {
            catalog: (*state).into_inner().catalog,
            selected: None,
        };
    }
    {
        let mut picker = state.borrow_mut();
        picker.hwnd = hwnd;
        picker.create(initial);
    }
    theme::title_bar(hwnd, dark);
    let icon = LoadImageW(
        instance,
        std::ptr::without_provenance(2),
        IMAGE_ICON,
        GetSystemMetricsForDpi(SM_CXSMICON, dpi),
        GetSystemMetricsForDpi(SM_CXSMICON, dpi),
        LR_SHARED,
    );
    SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, icon as isize);
    EnableWindow(owner, 0);
    ShowWindow(hwnd, SW_SHOW);
    SetFocus(GetDlgItem(hwnd, 1));
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
                    match GetDlgCtrlID(GetFocus()) {
                        26..=28 => {
                            PostMessageW(hwnd, WM_COMMAND, GetDlgCtrlID(GetFocus()) as usize, 0);
                        }
                        25 => state.borrow_mut().done = true,
                        _ => state.borrow_mut().accept(),
                    }
                    continue;
                }
                VK_DOWN if GetFocus() == GetDlgItem(hwnd, 1) => {
                    SetFocus(GetDlgItem(hwnd, 2));
                    continue;
                }
                VK_DELETE
                    if GetFocus() == GetDlgItem(hwnd, 2)
                        && IsWindowEnabled(GetDlgItem(hwnd, 28)) != 0 =>
                {
                    PostMessageW(hwnd, WM_COMMAND, 28, 0);
                    continue;
                }
                0x46 if GetKeyState(VK_CONTROL as i32) < 0 => {
                    SetFocus(GetDlgItem(hwnd, 1));
                    SendMessageW(GetDlgItem(hwnd, 1), EM_SETSEL, 0, -1);
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
    let result = state
        .borrow()
        .result
        .map(|i| state.borrow().catalog.entries[i].id.clone());
    EnableWindow(owner, 1);
    DestroyWindow(hwnd);
    SetActiveWindow(owner);
    DeleteObject(font);
    Choice {
        catalog: (*state).into_inner().catalog,
        selected: result,
    }
}
