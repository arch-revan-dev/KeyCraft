//! Dynamic HiDPI system tray icon generator (`RU` / `EN` / Paused) and native tray menu.

use keycraft_core_engine::Language;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::windows_mgr;

pub const TRAY_ID: &str = "keycraft-tray";
pub const MENU_TOGGLE_PAUSE: &str = "toggle_pause";
pub const MENU_SHOT_REGION: &str = "shot_region";
pub const MENU_SHOT_WINDOW: &str = "shot_window";
pub const MENU_SHOT_FULL: &str = "shot_full";
pub const MENU_SETTINGS: &str = "open_settings";
pub const MENU_QUIT: &str = "quit_app";

fn glyph_5x7(ch: char) -> [u8; 7] {
    match ch {
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'I' => [0b01100, 0b01100, 0b01100, 0b01100, 0b01100, 0b01100, 0b01100],
        _ => [0; 7],
    }
}

/// Render crisp 32x32 RGBA tray badge indicating current layout (`RU` / `EN`) or Pause state.
pub fn render_tray_icon(layout: Language, enabled: bool) -> Image<'static> {
    const SIZE: u32 = 32;
    let mut pixmap = Pixmap::new(SIZE, SIZE).expect("tray pixmap");

    // Rounded square background
    let mut pb = PathBuilder::new();
    let r = 6.0f32;
    let s = SIZE as f32;
    pb.move_to(r, 1.0);
    pb.line_to(s - r, 1.0);
    pb.quad_to(s - 1.0, 1.0, s - 1.0, r);
    pb.line_to(s - 1.0, s - r);
    pb.quad_to(s - 1.0, s - 1.0, s - r, s - 1.0);
    pb.line_to(r, s - 1.0);
    pb.quad_to(1.0, s - 1.0, 1.0, s - r);
    pb.line_to(1.0, r);
    pb.quad_to(1.0, 1.0, r, 1.0);
    pb.close();

    if let Some(path) = pb.finish() {
        let mut bg = Paint {
            anti_alias: true,
            ..Default::default()
        };
        if !enabled {
            bg.set_color(Color::from_rgba8(63, 63, 70, 255));
        } else if layout == Language::Ru {
            bg.set_color(Color::from_rgba8(37, 99, 235, 255)); // Calm cobalt #2563EB
        } else {
            bg.set_color(Color::from_rgba8(28, 28, 31, 255)); // Deep graphite #1C1C1F
        }
        pixmap.fill_path(&path, &bg, FillRule::Winding, Transform::identity(), None);

        let mut border = Paint {
            anti_alias: true,
            ..Default::default()
        };
        border.set_color(Color::from_rgba8(255, 255, 255, 42));
        pixmap.stroke_path(
            &path,
            &border,
            &Stroke {
                width: 1.0,
                ..Default::default()
            },
            Transform::identity(),
            None,
        );
    }

    let chars: [char; 2] = if !enabled {
        ['I', 'I']
    } else {
        match layout {
            Language::Ru => ['R', 'U'],
            Language::En => ['E', 'N'],
        }
    };

    // Draw crisp 2x scaled 5x7 glyphs centered inside 32x32
    let data = pixmap.data_mut();
    let start_x = 5usize;
    let start_y = 9usize;
    for (ci, ch) in chars.iter().enumerate() {
        let rows = glyph_5x7(*ch);
        let ox = start_x + ci * 12;
        for (ry, row) in rows.iter().enumerate() {
            for rx in 0..5usize {
                if (row >> (4 - rx)) & 1 == 1 {
                    for dy in 0..2usize {
                        for dx in 0..2usize {
                            let px = ox + rx * 2 + dx;
                            let py = start_y + ry * 2 + dy;
                            if px < SIZE as usize && py < SIZE as usize {
                                let idx = (py * SIZE as usize + px) * 4;
                                data[idx] = 248;
                                data[idx + 1] = 248;
                                data[idx + 2] = 250;
                                data[idx + 3] = 255;
                            }
                        }
                    }
                }
            }
        }
    }

    Image::new_owned(pixmap.take(), SIZE, SIZE)
}

pub fn build_tray_menu(
    app: &AppHandle,
    enabled: bool,
    screenshot_enabled: bool,
) -> tauri::Result<Menu<tauri::Wry>> {
    let pause_label = if enabled {
        "Приостановить"
    } else {
        "Возобновить"
    };
    let toggle_pause = MenuItem::with_id(app, MENU_TOGGLE_PAUSE, pause_label, true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let open_settings = MenuItem::with_id(app, MENU_SETTINGS, "Настройки…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Выход", true, None::<&str>)?;

    if screenshot_enabled {
        let shot_region =
            MenuItem::with_id(app, MENU_SHOT_REGION, "Скриншот области", true, None::<&str>)?;
        let shot_window =
            MenuItem::with_id(app, MENU_SHOT_WINDOW, "Скриншот окна", true, None::<&str>)?;
        let shot_full =
            MenuItem::with_id(app, MENU_SHOT_FULL, "Скриншот экрана", true, None::<&str>)?;
        let sep2 = PredefinedMenuItem::separator(app)?;
        Menu::with_items(
            app,
            &[
                &toggle_pause,
                &sep1,
                &shot_region,
                &shot_window,
                &shot_full,
                &sep2,
                &open_settings,
                &quit,
            ],
        )
    } else {
        Menu::with_items(app, &[&toggle_pause, &sep1, &open_settings, &quit])
    }
}

pub fn setup_tray(
    app: &AppHandle,
    layout: Language,
    enabled: bool,
    screenshot_enabled: bool,
) -> tauri::Result<TrayIcon> {
    let menu = build_tray_menu(app, enabled, screenshot_enabled)?;
    let icon = render_tray_icon(layout, enabled);

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip(format!(
            "KeyCraft — {} ({})",
            layout.label(),
            if enabled { "активен" } else { "пауза" }
        ))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_TOGGLE_PAUSE => {
                crate::toggle_enabled_from_backend(app);
            }
            MENU_SHOT_REGION => {
                windows_mgr::open_screenshot_overlay_async(app.clone(), "region".to_string());
            }
            MENU_SHOT_WINDOW => {
                windows_mgr::open_screenshot_overlay_async(app.clone(), "window".to_string());
            }
            MENU_SHOT_FULL => {
                windows_mgr::open_screenshot_overlay_async(app.clone(), "fullscreen".to_string());
            }
            MENU_SETTINGS => {
                let _ = windows_mgr::open_settings_window(app);
            }
            MENU_QUIT => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = windows_mgr::open_settings_window(tray.app_handle());
            }
        })
        .build(app)
}

/// Update tray icon & tooltip safely on the main UI thread (without rebuilding HMENU).
pub fn refresh_tray_icon(app: &AppHandle, layout: Language, enabled: bool) {
    let app_ui = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = app_ui.tray_by_id(TRAY_ID) {
            let _ = tray.set_icon(Some(render_tray_icon(layout, enabled)));
            let _ = tray.set_tooltip(Some(format!(
                "KeyCraft — {} ({})",
                layout.label(),
                if enabled { "активен" } else { "пауза" }
            )));
        }
    });
}

/// Update tray icon, tooltip, AND menu (when Pause/Resume or screenshot toggle changes) on the main UI thread.
pub fn refresh_tray_visuals(
    app: &AppHandle,
    layout: Language,
    enabled: bool,
    screenshot_enabled: bool,
) {
    let app_ui = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = app_ui.tray_by_id(TRAY_ID) {
            let _ = tray.set_icon(Some(render_tray_icon(layout, enabled)));
            let _ = tray.set_tooltip(Some(format!(
                "KeyCraft — {} ({})",
                layout.label(),
                if enabled { "активен" } else { "пауза" }
            )));
            if let Ok(menu) = build_tray_menu(&app_ui, enabled, screenshot_enabled) {
                let _ = tray.set_menu(Some(menu));
            }
        }
    });
}
