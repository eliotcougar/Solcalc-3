//! Main Win32 controls and event loop.
use crate::calc::{self, Basis, Error, Unit, CONCENTRATION, MASS, VOLUME};
use crate::{
    i18n,
    settings::{self, Language, Settings, Theme},
    substance_picker, substances, theme,
};
use std::{
    cell::RefCell,
    mem::{size_of, zeroed},
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{
        LibraryLoader::*,
        SystemServices::{SS_ENDELLIPSIS, SS_NOPREFIX},
    },
    UI::{Controls::*, HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

const UPDATE: u32 = WM_APP + 1;
const THEME_CHANGED: u32 = WM_APP + 2;
const SETTINGS_CHANGED: u32 = WM_APP + 3;
const BASIS_CHANGED: u32 = WM_APP + 4;
const EDIT_IDS: [&[i32]; 3] = [
    &[101, 102, 103, 104],
    &[300, 301, 302],
    &[500, 501, 502, 503, 504, 508, 510, 511],
];
const UNIT_IDS: [&[i32]; 3] = [
    &[202, 203, 204],
    &[400, 401, 402],
    &[600, 601, 603, 505, 610, 611],
];
struct FormState {
    values: Vec<(i32, String)>,
    selections: Vec<(i32, usize)>,
}
impl FormState {
    fn initial(mode: usize, basis: Basis) -> Self {
        let selections = match mode {
            0 => vec![
                (202, if basis == Basis::Molar { 1 } else { 0 }),
                (203, 1),
                (204, 1),
            ],
            1 => vec![
                (400, if basis == Basis::Molar { 1 } else { 0 }),
                (401, if basis == Basis::Molar { 1 } else { 0 }),
                (402, 1),
            ],
            _ => vec![
                (600, 1),
                (601, 2),
                (603, if basis == Basis::Molar { 2 } else { 0 }),
                (505, 2),
                (610, 2),
                (611, 1),
            ],
        };
        Self {
            values: EDIT_IDS[mode]
                .iter()
                .map(|id| (*id, if *id == 504 { "10" } else { "" }.into()))
                .collect(),
            selections,
        }
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn low(v: usize) -> u16 {
    (v & 0xffff) as u16
}
fn high(v: usize) -> u16 {
    ((v >> 16) & 0xffff) as u16
}

struct Element {
    hwnd: HWND,
    mode: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: usize,
    source: Option<String>,
}
struct Ui {
    hwnd: HWND,
    dpi: u32,
    mode: i32,
    elements: Vec<Element>,
    fonts: [HFONT; 3],
    background: HBRUSH,
    panel: HBRUSH,
    ready: bool,
    settings: Settings,
    settings_path: Option<std::path::PathBuf>,
    dark: Option<bool>,
    combo_options: Vec<(HWND, Vec<String>)>,
    bases: [Basis; 3],
    basis_states: [[Option<FormState>; 2]; 3],
    substances: [Option<String>; 3],
    catalog: Option<substances::Catalog>,
}
impl Ui {
    fn new() -> Self {
        let settings_path = settings::path();
        let settings = settings_path
            .as_ref()
            .map(|p| Settings::load(p))
            .unwrap_or_default();
        theme::set_dark(settings.theme.is_dark(theme::system_is_dark()));
        let colors = theme::colors();
        // SAFETY: solid brushes have no borrowed resources and are released by Drop.
        unsafe {
            Self {
                hwnd: null_mut(),
                dpi: 96,
                mode: 0,
                elements: vec![],
                fonts: [null_mut(); 3],
                background: CreateSolidBrush(colors.background),
                panel: CreateSolidBrush(colors.panel),
                ready: false,
                settings,
                settings_path,
                dark: None,
                combo_options: vec![],
                bases: [Basis::Molar; 3],
                basis_states: std::array::from_fn(|_| [None, None]),
                substances: std::array::from_fn(|_| None),
                catalog: None,
            }
        }
    }
    fn s(&self, n: i32) -> i32 {
        (n as i64 * self.dpi as i64 / 96) as i32
    }
    unsafe fn fonts(&mut self) {
        for font in &mut self.fonts {
            if !font.is_null() {
                DeleteObject(*font);
            }
        }
        for (index, (height, weight)) in [(15, 400), (28, 600), (17, 600)].into_iter().enumerate() {
            self.fonts[index] = CreateFontW(
                -self.s(height),
                0,
                0,
                0,
                weight,
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
        }
        for el in &self.elements {
            SendMessageW(el.hwnd, WM_SETFONT, self.fonts[el.font] as usize, 1);
        }
    }
    #[allow(clippy::too_many_arguments)]
    unsafe fn control(
        &mut self,
        class: &str,
        text: &str,
        id: i32,
        mode: i32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        style: u32,
        font: usize,
    ) -> HWND {
        let source = ((class == "STATIC" && (id == 0 || id == 20)) || class == "BUTTON")
            .then(|| text.to_owned());
        let text = i18n::text(self.settings.language, text);
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
        assert!(!hwnd.is_null(), "Could not create a native control");
        SendMessageW(hwnd, WM_SETFONT, self.fonts[font] as usize, 1);
        if class == "EDIT" {
            theme::install_edit(hwnd);
        }
        self.elements.push(Element {
            hwnd,
            mode,
            x,
            y,
            w,
            h,
            font,
            source,
        });
        hwnd
    }
    #[allow(clippy::too_many_arguments)]
    unsafe fn label(&mut self, text: &str, mode: i32, x: i32, y: i32, w: i32, h: i32, font: usize) {
        self.control("STATIC", text, 0, mode, x, y, w, h, 0, font);
    }
    #[allow(clippy::too_many_arguments)]
    unsafe fn combo(
        &mut self,
        id: i32,
        mode: i32,
        x: i32,
        y: i32,
        w: i32,
        options: &[&str],
        initial: usize,
    ) {
        let h = self.control(
            "COMBOBOX",
            "",
            id,
            mode,
            x,
            y,
            w,
            300,
            WS_TABSTOP
                | WS_VSCROLL
                | CBS_DROPDOWNLIST as u32
                | CBS_OWNERDRAWFIXED as u32
                | CBS_HASSTRINGS as u32,
            0,
        );
        theme::install_combo(h);
        for option in options {
            let label = i18n::numbers(
                self.settings.language,
                i18n::text(self.settings.language, option),
            );
            SendMessageW(h, CB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
        }
        self.combo_options
            .push((h, options.iter().map(|s| (*s).to_owned()).collect()));
        SendMessageW(h, CB_SETITEMHEIGHT, usize::MAX, self.s(24) as isize);
        SendMessageW(h, CB_SETCURSEL, initial, 0);
    }
    unsafe fn units(&mut self, id: i32, mode: i32, x: i32, y: i32, units: &[Unit], initial: usize) {
        self.combo(
            id,
            mode,
            x,
            y,
            88,
            &units.iter().map(|u| u.label()).collect::<Vec<_>>(),
            initial,
        );
    }
    unsafe fn field(
        &mut self,
        label: &str,
        id: i32,
        mode: i32,
        y: i32,
        units: Option<(&[Unit], usize)>,
    ) {
        self.label(label, mode, 28, y + 4, 190, 25, 0);
        let h = self.control(
            "EDIT",
            "",
            id,
            mode,
            220,
            y,
            140,
            28,
            WS_TABSTOP | ES_AUTOHSCROLL as u32,
            0,
        );
        SendMessageW(h, EM_SETLIMITTEXT, 80, 0);
        if let Some((units, initial)) = units {
            self.units(id + 100, mode, 370, y, units, initial);
        }
    }
    unsafe fn get(&self, id: i32) -> HWND {
        GetDlgItem(self.hwnd, id)
    }
    unsafe fn set(&self, id: i32, text: &str) {
        SetWindowTextW(self.get(id), wide(text).as_ptr());
    }
    unsafe fn text(&self, id: i32) -> String {
        let h = self.get(id);
        let mut text = vec![0u16; GetWindowTextLengthW(h).max(0) as usize + 1];
        let len = GetWindowTextW(h, text.as_mut_ptr(), text.len() as i32);
        String::from_utf16_lossy(&text[..len.max(0) as usize])
    }
    unsafe fn selected(&self, id: i32) -> usize {
        SendMessageW(self.get(id), CB_GETCURSEL, 0, 0).max(0) as usize
    }
    unsafe fn select(&self, id: i32, index: usize) {
        SendMessageW(self.get(id), CB_SETCURSEL, index, 0);
    }
    unsafe fn unit(&self, id: i32, units: &[Unit]) -> Unit {
        units[self.selected(id).min(units.len() - 1)]
    }
    unsafe fn value(&self, id: i32, label: &str) -> Result<f64, Error> {
        let v = calc::parse(&self.text(id), label)?
            .ok_or_else(|| Error(format!("Enter {} to see the result.", label.to_lowercase())))?;
        calc::positive(v, label)
    }
    unsafe fn quantity(&self, id: i32, label: &str, units: &[Unit]) -> Result<f64, Error> {
        self.unit(id + 100, units).to_base(self.value(id, label)?)
    }
    unsafe fn create(&mut self) {
        self.dpi = GetDpiForWindow(self.hwnd).max(96);
        self.fonts();
        for (i, title) in ["&1  Mass / volume", "&2  Dilution", "&3  Stock preparation"]
            .iter()
            .enumerate()
        {
            self.control(
                "BUTTON",
                title,
                10 + i as i32,
                -1,
                28 + i as i32 * 150,
                20,
                144,
                34,
                WS_TABSTOP
                    | BS_AUTORADIOBUTTON as u32
                    | BS_PUSHLIKE as u32
                    | if i == 0 { WS_GROUP } else { 0 },
                0,
            );
        }
        self.combo(
            30,
            -1,
            602,
            23,
            170,
            &["Light", "Dark", "System"],
            self.settings.theme.index(),
        );
        self.combo(
            31,
            -1,
            788,
            23,
            70,
            &["EN", "RU"],
            self.settings.language.index(),
        );
        self.substance_row(26, 0, 80);
        self.field("&Molar mass", 101, 0, 124, None);
        self.combo(201, 0, 370, 124, 88, &["g/mol", "U/mg"], 0);
        self.field("&Concentration", 102, 0, 170, Some((CONCENTRATION, 1)));
        self.field("&Volume", 103, 0, 216, Some((VOLUME, 1)));
        self.field("&Mass", 104, 0, 262, Some((MASS, 1)));

        self.label("Concentration basis", 1, 28, 84, 190, 25, 0);
        self.combo(303, 1, 220, 80, 238, &["g/mol", "U/mg"], 0);
        self.field(
            "&Stock concentration",
            300,
            1,
            126,
            Some((CONCENTRATION, 1)),
        );
        self.field(
            "&Desired concentration",
            301,
            1,
            172,
            Some((CONCENTRATION, 1)),
        );
        self.field("&Final volume", 302, 1, 218, Some((VOLUME, 1)));

        self.field("Final sample &volume", 500, 2, 80, Some((VOLUME, 1)));
        self.field("Stock &per sample", 501, 2, 116, Some((VOLUME, 2)));
        self.field("&Number of samples", 502, 2, 152, None);
        self.field(
            "Final &concentration",
            503,
            2,
            188,
            Some((CONCENTRATION, 2)),
        );
        self.field("&Extra volume", 504, 2, 224, None);
        self.label("%", 2, 370, 228, 60, 24, 0);
        self.set(504, "10");
        self.label("Round batch up to", 2, 28, 264, 190, 26, 0);
        self.combo(
            505,
            2,
            220,
            260,
            238,
            &["No rounding", "10 µL", "100 µL / 0.1 mL", "1 mL", "0.01 L"],
            2,
        );
        self.label("Preparation", 2, 28, 306, 430, 26, 2);
        self.substance_row(27, 2, 342);
        self.field("Molar &mass", 508, 2, 386, None);
        self.combo(608, 2, 370, 386, 88, &["g/mol", "U/mg"], 0);
        self.field("&Batch volume", 510, 2, 422, Some((VOLUME, 2)));
        self.field("M&ass", 511, 2, 458, Some((MASS, 1)));

        self.control("STATIC", "RESULT", 20, -1, 508, 92, 280, 26, 0, 2);
        self.control("STATIC", "", 21, -1, 508, 130, 280, 56, 0, 1);
        self.control("STATIC", "", 22, -1, 508, 196, 280, 120, 0, 0);
        self.control(
            "BUTTON",
            "C&lear inputs",
            25,
            -1,
            650,
            330,
            130,
            32,
            WS_TABSTOP,
            0,
        );
        self.ready = true;
        self.apply_language();
        self.apply_theme();
        self.load_icons();
        self.switch(0);
    }
    unsafe fn substance_row(&mut self, id: i32, mode: i32, y: i32) {
        self.control(
            "BUTTON",
            "&Substance…",
            id,
            mode,
            28,
            y,
            136,
            30,
            WS_TABSTOP,
            0,
        );
        self.control(
            "STATIC",
            "",
            id + 700,
            mode,
            174,
            y - 1,
            284,
            22,
            SS_NOPREFIX | SS_ENDELLIPSIS,
            0,
        );
        self.control(
            "STATIC",
            "",
            id + 800,
            mode,
            174,
            y + 20,
            284,
            21,
            SS_NOPREFIX,
            0,
        );
    }
    unsafe fn use_substance(&mut self, mode: usize, id: String) {
        let Some(entry) = self.catalog.as_ref().and_then(|c| c.find(&id)) else {
            return;
        };
        let mass = entry.molar_mass.to_string();
        let mass = i18n::numbers(self.settings.language, &mass);
        // A database choice is shared by the two preparation calculators.
        // Their other molar inputs and their separate activity forms remain
        // independent. Never put a formula mass in a U/mg coefficient field.
        for (target, basis_id, coefficient_id) in [(0, 201, 101), (2, 608, 508)] {
            self.select(basis_id, 0);
            self.change_basis(basis_id);
            self.set(coefficient_id, &mass);
            self.substances[target] = Some(id.clone());
        }
        self.update();
        let coefficient_id = if mode == 0 { 101 } else { 508 };
        SetFocus(self.get(coefficient_id));
        SendMessageW(self.get(coefficient_id), EM_SETSEL, 0, -1);
    }
    unsafe fn refresh_substances(&mut self) {
        for (mode, coefficient, label) in [(0, 101, 726), (2, 508, 727)] {
            let mut text = String::new();
            let mut formula = String::new();
            if self.substances[mode]
                .as_ref()
                .is_some_and(|id| self.catalog.as_ref().is_some_and(|c| c.find(id).is_none()))
            {
                self.substances[mode] = None;
            }
            if self.bases[mode] == Basis::Molar {
                if let Some(entry) = self.substances[mode]
                    .as_ref()
                    .and_then(|id| self.catalog.as_ref().and_then(|c| c.find(id)))
                {
                    if calc::parse(&self.text(coefficient), "Molar mass")
                        .ok()
                        .flatten()
                        == Some(entry.molar_mass)
                    {
                        text = entry.name(self.settings.language).to_owned();
                        formula = entry.formula_display();
                    } else {
                        self.substances[mode] = None;
                    }
                }
            }
            self.set(label, &text);
            self.set(label + 100, &formula);
        }
    }
    unsafe fn load_icons(&self) {
        let size = GetSystemMetricsForDpi(SM_CXSMICON, self.dpi);
        let icon = LoadImageW(
            GetModuleHandleW(null()),
            std::ptr::without_provenance(2),
            IMAGE_ICON,
            size,
            size,
            LR_SHARED,
        );
        if !icon.is_null() {
            SendMessageW(self.hwnd, WM_SETICON, ICON_SMALL as usize, icon as isize);
        }
    }
    unsafe fn apply_theme(&mut self) {
        let dark = self.settings.theme.is_dark(theme::system_is_dark());
        if self.dark == Some(dark) {
            return;
        }
        self.dark = Some(dark);
        theme::set_dark(dark);
        let colors = theme::colors();
        DeleteObject(self.background);
        DeleteObject(self.panel);
        self.background = CreateSolidBrush(colors.background);
        self.panel = CreateSolidBrush(colors.panel);
        theme::title_bar(self.hwnd, dark);
        for element in &self.elements {
            let mut class = [0u16; 32];
            let len = GetClassNameW(element.hwnd, class.as_mut_ptr(), class.len() as i32);
            if String::from_utf16_lossy(&class[..len.max(0) as usize]) == "Edit" {
                if dark {
                    SetWindowTheme(element.hwnd, wide("").as_ptr(), wide("").as_ptr());
                } else {
                    SetWindowTheme(element.hwnd, null(), null());
                }
            }
            InvalidateRect(element.hwnd, null(), 1);
        }
        RedrawWindow(
            self.hwnd,
            null(),
            null_mut(),
            RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_FRAME,
        );
    }
    unsafe fn apply_language(&mut self) {
        let language = self.settings.language;
        if self.catalog.is_some() {
            let loaded = if let Some(path) = crate::user_db::path() {
                substances::Catalog::load(language.code(), &path)
            } else {
                substances::Catalog::bundled(language.code())
            };
            match loaded {
                Ok(c) => self.catalog = Some(c),
                Err(error) => {
                    MessageBoxW(
                        self.hwnd,
                        wide(&format!(
                            "{}\n{}",
                            language.choose(
                                "Could not reload the user database.",
                                "Не удалось обновить базу пользователя."
                            ),
                            error
                        ))
                        .as_ptr(),
                        wide("Solcalc").as_ptr(),
                        MB_OK | MB_ICONERROR,
                    );
                }
            }
        }
        SetWindowTextW(
            self.hwnd,
            wide(i18n::text(language, "Solcalc — Solution Calculator")).as_ptr(),
        );
        for element in &self.elements {
            if let Some(source) = &element.source {
                SetWindowTextW(element.hwnd, wide(i18n::text(language, source)).as_ptr());
            }
        }
        for (hwnd, options) in &self.combo_options {
            if GetDlgCtrlID(*hwnd) == 31 {
                continue;
            }
            let selected = SendMessageW(*hwnd, CB_GETCURSEL, 0, 0).max(0);
            SendMessageW(*hwnd, CB_RESETCONTENT, 0, 0);
            for option in options {
                let text = i18n::numbers(language, i18n::text(language, option));
                SendMessageW(*hwnd, CB_ADDSTRING, 0, wide(&text).as_ptr() as isize);
            }
            SendMessageW(*hwnd, CB_SETCURSEL, selected as usize, 0);
        }
        self.update();
        InvalidateRect(self.hwnd, null(), 1);
    }
    unsafe fn change_setting(&mut self, id: i32) {
        if id == 30 {
            self.settings.theme = Theme::from_index(self.selected(30));
            self.apply_theme();
        } else {
            self.settings.language = if self.selected(31) == 1 {
                Language::Ru
            } else {
                Language::En
            };
            self.apply_language();
        }
        let result = self
            .settings_path
            .as_ref()
            .ok_or_else(|| std::io::Error::other("LOCALAPPDATA is unavailable"))
            .and_then(|p| self.settings.save(p));
        if let Err(error) = result {
            let language = self.settings.language;
            MessageBoxW(
                self.hwnd,
                wide(&format!(
                    "{}\n{error}",
                    language.choose(
                        "Could not save settings.",
                        "Не удалось сохранить настройки."
                    )
                ))
                .as_ptr(),
                wide("Solcalc").as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
    }
    unsafe fn replace_units(&mut self, id: i32, units: &[Unit]) {
        let hwnd = self.get(id);
        let options: Vec<String> = units.iter().map(|u| u.label().to_owned()).collect();
        SendMessageW(hwnd, CB_RESETCONTENT, 0, 0);
        for option in &options {
            SendMessageW(hwnd, CB_ADDSTRING, 0, wide(option).as_ptr() as isize);
        }
        if let Some((_, original)) = self.combo_options.iter_mut().find(|(h, _)| *h == hwnd) {
            *original = options;
        }
    }
    unsafe fn change_basis(&mut self, id: i32) {
        let mode = match id {
            201 => 0,
            303 => 1,
            608 => 2,
            _ => return,
        };
        let basis = Basis::from_index(self.selected(id));
        let previous = self.bases[mode];
        if basis == previous {
            return;
        }
        self.basis_states[mode][previous.index()] = Some(FormState {
            values: EDIT_IDS[mode]
                .iter()
                .map(|id| (*id, self.text(*id)))
                .collect(),
            selections: UNIT_IDS[mode]
                .iter()
                .map(|id| (*id, self.selected(*id)))
                .collect(),
        });
        self.bases[mode] = basis;
        for unit_id in match mode {
            0 => &[202][..],
            1 => &[400, 401][..],
            _ => &[603][..],
        } {
            self.replace_units(*unit_id, basis.concentration_units());
        }
        let coefficient_id = match mode {
            0 => Some(101),
            2 => Some(508),
            _ => None,
        };
        if let Some(coefficient_id) = coefficient_id {
            let hwnd = self.get(coefficient_id);
            if let Some(index) = self.elements.iter().position(|e| e.hwnd == hwnd) {
                let source = if basis == Basis::Activity {
                    "&Specific activity"
                } else if mode == 0 {
                    "&Molar mass"
                } else {
                    "Molar &mass"
                };
                let label = &mut self.elements[index - 1];
                label.source = Some(source.to_owned());
                SetWindowTextW(
                    label.hwnd,
                    wide(i18n::text(self.settings.language, source)).as_ptr(),
                );
            }
        }
        let state = self.basis_states[mode][basis.index()]
            .take()
            .unwrap_or_else(|| FormState::initial(mode, basis));
        for (id, value) in state.values {
            self.set(id, &value);
        }
        for (id, selected) in state.selections {
            self.select(id, selected);
        }
        self.update();
    }
    unsafe fn switch(&mut self, mode: i32) {
        self.mode = mode;
        for el in &self.elements {
            ShowWindow(
                el.hwnd,
                if el.mode == -1 || el.mode == mode {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
        }
        CheckRadioButton(self.hwnd, 10, 12, 10 + mode);
        if IsZoomed(self.hwnd) == 0 {
            let mut rect = zeroed();
            GetWindowRect(self.hwnd, &mut rect);
            SetWindowPos(
                self.hwnd,
                null_mut(),
                0,
                0,
                rect.right - rect.left,
                self.s(self.window_height()),
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        self.update();
        self.layout();
        SetFocus(self.get([101, 300, 500][mode as usize]));
        InvalidateRect(self.hwnd, null(), 1);
    }
    fn window_height(&self) -> i32 {
        if self.mode == 2 {
            598
        } else if self.mode == 0 {
            464
        } else {
            420
        }
    }
    unsafe fn layout(&self) {
        if !self.ready {
            return;
        }
        let mut rect = zeroed();
        GetClientRect(self.hwnd, &mut rect);
        let width = rect.right * 96 / self.dpi as i32;
        let height = rect.bottom * 96 / self.dpi as i32;
        for el in &self.elements {
            let id = GetDlgCtrlID(el.hwnd);
            let (x, y, w, h) = match id {
                20 | 21 => (508, el.y, width - 536, el.h),
                22 => (508, el.y, width - 536, height - el.y - 78),
                25 => (width - 158, height - 48, el.w, el.h),
                30 => (width - 284, el.y, el.w, el.h),
                31 => (width - 98, el.y, el.w, el.h),
                _ => (el.x, el.y, el.w, el.h),
            };
            MoveWindow(el.hwnd, self.s(x), self.s(y), self.s(w), self.s(h), 1);
        }
    }
    unsafe fn optional_quantity(
        &self,
        id: i32,
        label: &str,
        units: &[Unit],
    ) -> Result<Option<f64>, Error> {
        calc::parse(&self.text(id), label)?
            .map(|v| self.unit(id + 100, units).to_base(v))
            .transpose()
    }
    unsafe fn update(&mut self) {
        if !self.ready {
            return;
        }
        self.refresh_substances();
        let calculated = match self.mode {
            0 => self.main_result(),
            1 => self.dilution_result(),
            _ => self.stock_result(),
        };
        match calculated {
            Ok((primary, detail)) => {
                self.set(21, &i18n::numbers(self.settings.language, &primary));
                self.set(22, &i18n::numbers(self.settings.language, &detail));
            }
            Err(error) => {
                let incomplete =
                    error.0.starts_with("Enter ") && error.0.ends_with("to see the result.");
                self.set(21, "");
                let detail = if incomplete {
                    String::new()
                } else {
                    i18n::error(self.settings.language, &error.0)
                };
                self.set(22, &detail);
            }
        }
    }
    unsafe fn main_result(&self) -> Result<(String, String), Error> {
        let language = self.settings.language;
        let basis = self.bases[0];
        let activity = basis == Basis::Activity;
        let mm = calc::parse(&self.text(101), basis.coefficient_label())?;
        let conc = self.optional_quantity(102, "Concentration", basis.concentration_units())?;
        let vol = self.optional_quantity(103, "Volume", VOLUME)?;
        let mass = self.optional_quantity(104, "Mass", MASS)?;
        Ok(match calc::solve_solution(basis, mm, conc, vol, mass)? {
            Some(calc::MissingQuantity::Mass(v)) => (
                calc::display_mass(v),
                i18n::text(
                    language,
                    if activity {
                        "Required protein mass\n\nm = C × V / SA"
                    } else {
                        "Required solute mass\n\nm = MW × C × V"
                    },
                )
                .into(),
            ),
            Some(calc::MissingQuantity::Volume(v)) => (
                calc::display_volume(v),
                i18n::text(
                    language,
                    if activity {
                        "Required final volume\n\nV = m × SA / C"
                    } else {
                        "Required final volume\n\nV = m / (MW × C)"
                    },
                )
                .into(),
            ),
            Some(calc::MissingQuantity::Concentration(v)) => (
                basis.display_concentration(v),
                i18n::text(
                    language,
                    if activity {
                        "Activity concentration\n\nC = m × SA / V"
                    } else {
                        "Concentration\n\nC = m / (MW × V)"
                    },
                )
                .into(),
            ),
            Some(calc::MissingQuantity::MolarMass(v)) => (
                format!("{} g/mol", calc::number(v)),
                i18n::text(language, "Molar mass\n\nMW = m / (C × V)").into(),
            ),
            Some(calc::MissingQuantity::SpecificActivity(v)) => (
                format!("{} U/mg", calc::number(v)),
                i18n::text(language, "Specific activity\n\nSA = C × V / m").into(),
            ),
            None => (String::new(), String::new()),
        })
    }
    unsafe fn dilution_result(&self) -> Result<(String, String), Error> {
        let language = self.settings.language;
        let units = self.bases[1].concentration_units();
        let stock = self.quantity(300, "Stock concentration", units)?;
        let desired = self.quantity(301, "Desired concentration", units)?;
        let vol = self.quantity(302, "Final volume", VOLUME)?;
        let d = calc::dilute(stock, desired, vol)?;
        Ok((
            format!(
                "{} {}",
                calc::display_volume(d.stock),
                language.choose("stock", "стока")
            ),
            format!(
                "+ {} {}\n{}: {}\n{}: {}×\n\nC₁ × V₁ = C₂ × V₂",
                calc::display_volume(d.solvent),
                language.choose("solvent", "растворителя"),
                language.choose("Final volume", "Конечный объём"),
                calc::display_volume(vol),
                language.choose("Dilution factor", "Разведение"),
                calc::number(d.factor)
            ),
        ))
    }
    unsafe fn stock_result(&self) -> Result<(String, String), Error> {
        let language = self.settings.language;
        let basis = self.bases[2];
        let sample = self.quantity(500, "Final sample volume", VOLUME)?;
        let addition = self.quantity(501, "Stock per sample", VOLUME)?;
        let n = self.value(502, "Number of samples")?;
        let conc = self.quantity(503, "Final concentration", basis.concentration_units())?;
        let extra = calc::parse(&self.text(504), "Extra volume")?.unwrap_or(0.0);
        let step = [0.0, 1e-5, 1e-4, 1e-3, 1e-2][self.selected(505).min(4)];
        let s = calc::stock(sample, addition, n, conc, extra, step)?;
        let mut detail = if language == Language::En {
            format!(
                "Prepare at least {}\nMinimum for {} samples: {}\nExtra volume: {}%{}",
                calc::display_volume(s.batch_volume),
                calc::number(n),
                calc::display_volume(s.minimum_volume),
                calc::number(extra),
                if step > 0.0 { "; rounded up" } else { "" }
            )
        } else {
            format!(
                "Приготовить не менее {}\nМинимум: {} (образцов: {})\nДоп. объём: {}%{}",
                calc::display_volume(s.batch_volume),
                calc::display_volume(s.minimum_volume),
                calc::number(n),
                calc::number(extra),
                if step > 0.0 {
                    "; округлено вверх"
                } else {
                    ""
                }
            )
        };
        if !self.text(508).trim().is_empty() {
            let preparation = (|| -> Result<String, Error> {
                let mm = self.value(508, basis.coefficient_label())?;
                let vol = self.optional_quantity(510, "Batch volume", VOLUME)?;
                let mass = self.optional_quantity(511, "Mass", MASS)?;
                let prep =
                    calc::prepare_stock(basis, mm, s.concentration, s.batch_volume, vol, mass)?;
                let warning = if calc::materially_less(prep.volume, s.batch_volume) {
                    language.choose(
                        "\nLess than the planned batch.",
                        "\nМеньше запланированного объёма.",
                    )
                } else {
                    ""
                };
                Ok(format!(
                    "{} {} {} {}{}",
                    calc::display_mass(prep.mass),
                    language.choose("in", "в"),
                    calc::display_volume(prep.volume),
                    language.choose("final volume", "конечного объёма"),
                    warning
                ))
            })();
            detail.push_str(&format!(
                "\n\n{}\n{}",
                language.choose("Preparation", "Приготовление"),
                match preparation {
                    Ok(text) => text,
                    Err(e) => i18n::error(language, &e.0),
                }
            ));
        }
        Ok((
            format!(
                "{} {}",
                basis.display_concentration(s.concentration),
                language.choose("stock", "сток")
            ),
            detail,
        ))
    }
    unsafe fn clear(&mut self) {
        self.substances[self.mode as usize] = None;
        let ids: &[i32] = match self.mode {
            0 => &[101, 102, 103, 104],
            1 => &[300, 301, 302],
            _ => &[500, 501, 502, 503, 508, 510, 511],
        };
        for id in ids {
            self.set(*id, "");
        }
        self.update();
        SetFocus(self.get([101, 300, 500][self.mode as usize]));
    }
    unsafe fn paint(&mut self) {
        let mut ps = zeroed();
        let dc = BeginPaint(self.hwnd, &mut ps);
        let mut rect = zeroed();
        GetClientRect(self.hwnd, &mut rect);
        FillRect(dc, &rect, self.background);
        let panel = RECT {
            left: self.s(484),
            top: self.s(76),
            right: rect.right - self.s(28),
            bottom: rect.bottom - self.s(66),
        };
        FillRect(dc, &panel, self.panel);
        EndPaint(self.hwnd, &ps);
    }
}
impl Drop for Ui {
    fn drop(&mut self) {
        // All HWNDs have been destroyed before their shared GDI resources.
        unsafe {
            for font in self.fonts {
                if !font.is_null() {
                    DeleteObject(font);
                }
            }
            DeleteObject(self.background);
            DeleteObject(self.panel);
        }
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if theme::redirect_modal(hwnd, message, w) {
        return 0;
    }
    if message == WM_NCCREATE {
        let create = &*(l as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    // These messages can arrive synchronously while another callback owns the
    // state. Handle them without borrowing it: default WM_CLOSE destroys the
    // window and must still deliver WM_DESTROY / WM_NCDESTROY.
    match message {
        WM_DESTROY => {
            PostQuitMessage(0);
            return 0;
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            return DefWindowProcW(hwnd, message, w, l);
        }
        WM_CLOSE => return DefWindowProcW(hwnd, message, w, l),
        _ => {}
    }
    if let Some(result) = theme::color_control(message, w, l) {
        return result;
    }
    if message == WM_NOTIFY {
        if let Some(result) = theme::button_custom_draw(l) {
            return result;
        }
    }
    if message == WM_DRAWITEM && theme::draw_combo(l) {
        return 1;
    }
    if message == WM_MEASUREITEM && theme::measure_combo(hwnd, l) {
        return 1;
    }
    if matches!(message, WM_SETTINGCHANGE | WM_THEMECHANGED) {
        PostMessageW(hwnd, THEME_CHANGED, 0, 0);
        return DefWindowProcW(hwnd, message, w, l);
    }
    // Default processing may synchronously send WM_SIZE (maximize/restore),
    // WM_DESTROY, or focus messages. Do not hold a state borrow across it.
    if !matches!(
        message,
        WM_COMMAND
            | UPDATE
            | THEME_CHANGED
            | SETTINGS_CHANGED
            | BASIS_CHANGED
            | WM_SIZE
            | WM_GETMINMAXINFO
            | WM_DPICHANGED
            | WM_PAINT
            | WM_ERASEBKGND
    ) {
        return DefWindowProcW(hwnd, message, w, l);
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<Ui>;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, message, w, l);
    }
    if message == WM_COMMAND && matches!(low(w), 26 | 27) && high(w) == BN_CLICKED as u16 {
        let mode = if low(w) == 26 { 0 } else { 2 };
        let (language, dark, initial) = {
            let ui = (*ptr).borrow();
            (
                ui.settings.language,
                ui.dark.unwrap_or(false),
                ui.substances[mode].clone(),
            )
        };
        let choice = substance_picker::choose(hwnd, language, dark, initial);
        let mut ui = (*ptr).borrow_mut();
        ui.catalog = Some(choice.catalog);
        if let Some(id) = choice.selected {
            ui.use_substance(mode, id);
        } else {
            ui.update();
        }
        return 0;
    }
    // Win32 can call back synchronously during control updates. A failed
    // borrow delegates nested notifications instead of creating aliased &mut.
    let Ok(mut ui) = (*ptr).try_borrow_mut() else {
        return DefWindowProcW(hwnd, message, w, l);
    };
    match message {
        WM_COMMAND => {
            let id = low(w) as i32;
            match id {
                10..=12 if high(w) == BN_CLICKED as u16 => ui.switch(id - 10),
                25 => ui.clear(),
                30 | 31 if high(w) == CBN_SELCHANGE as u16 => {
                    PostMessageW(hwnd, SETTINGS_CHANGED, id as usize, 0);
                }
                201 | 303 | 608 if high(w) == CBN_SELCHANGE as u16 => {
                    PostMessageW(hwnd, BASIS_CHANGED, id as usize, 0);
                }
                _ if high(w) == EN_CHANGE as u16 || high(w) == CBN_SELCHANGE as u16 => {
                    PostMessageW(hwnd, UPDATE, 0, 0);
                }
                _ => {}
            }
            0
        }
        UPDATE => {
            ui.update();
            0
        }
        THEME_CHANGED => {
            if ui.ready {
                ui.apply_theme();
            }
            0
        }
        SETTINGS_CHANGED => {
            ui.change_setting(w as i32);
            0
        }
        BASIS_CHANGED => {
            ui.change_basis(w as i32);
            0
        }
        WM_SIZE => {
            ui.layout();
            InvalidateRect(hwnd, null(), 1);
            0
        }
        WM_GETMINMAXINFO => {
            let info = &mut *(l as *mut MINMAXINFO);
            info.ptMinTrackSize = POINT {
                x: ui.s(870),
                y: ui.s(ui.window_height()),
            };
            0
        }
        WM_DPICHANGED => {
            ui.dpi = low(w) as u32;
            ui.fonts();
            ui.load_icons();
            let rect = &*(l as *const RECT);
            SetWindowPos(
                hwnd,
                null_mut(),
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            ui.layout();
            InvalidateRect(hwnd, null(), 1);
            0
        }
        WM_PAINT => {
            ui.paint();
            0
        }
        WM_ERASEBKGND => 1,
        _ => 0,
    }
}

pub fn run() {
    let state = Box::new(RefCell::new(Ui::new()));
    // SAFETY: The state Box stays at a stable address until the message loop
    // ends. HWND callbacks stay on this thread. Strings live through each FFI
    // call; Windows copies them. No callback unwinds during normal operation.
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let common = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES,
        };
        InitCommonControlsEx(&common);
        let instance = GetModuleHandleW(null());
        let class = wide("Solcalc3.NativeWindow");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hIcon: LoadIconW(instance, std::ptr::without_provenance(1)),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            lpszClassName: class.as_ptr(),
            ..zeroed()
        };
        if RegisterClassW(&wc) == 0 {
            return;
        }
        let dpi = GetDpiForSystem().max(96);
        let width = 900 * dpi as i32 / 96;
        let height = 464 * dpi as i32 / 96;
        let hwnd = CreateWindowExW(
            WS_EX_CONTROLPARENT,
            class.as_ptr(),
            wide("Solcalc — Solution Calculator").as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            null_mut(),
            null_mut(),
            instance,
            state.as_ref() as *const RefCell<Ui> as *const _,
        );
        if hwnd.is_null() {
            return;
        }
        {
            let mut ui = state.borrow_mut();
            ui.hwnd = hwnd;
            ui.create();
        }
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        let mut message = zeroed();
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
            let ctrl = GetKeyState(VK_CONTROL as i32) < 0;
            if message.message == WM_KEYDOWN && ctrl && (0x31..=0x33).contains(&message.wParam) {
                state.borrow_mut().switch(message.wParam as i32 - 0x31);
            } else if IsDialogMessageW(hwnd, &message) == 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}
