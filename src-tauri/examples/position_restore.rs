//! Native regression check: cargo run -p desktop-pet --example position_restore
//! Requires a desktop session. Uses hidden windows and temporary settings only.
#[allow(dead_code)]
#[path = "../src/window.rs"]
mod window;

use pet_core::{geometry::Point, manifest::RenderConfig, settings::Settings};
use std::{sync::mpsc, time::{Duration, Instant}};
use tauri::{WebviewUrl, WebviewWindowBuilder};

fn check(app: &tauri::AppHandle) -> Result<(), String> {
    let path = std::env::temp_dir().join(format!("desktop-pet-position-{}.json", std::process::id()));
    let result = (|| {
        let mut saved = None;
        for restart in 0..3 {
            let requested = saved;
            let (tx, rx) = mpsc::channel();
            let handle = app.clone();
            app.run_on_main_thread(move || {
                let result = (|| {
                    let pet = WebviewWindowBuilder::new(&handle, format!("restore-{restart}"), WebviewUrl::External("about:blank".parse().unwrap()))
                        .visible(false).decorations(false).inner_size(256.0, 256.0)
                        .build().map_err(|e| e.to_string())?;
                    let work = pet.primary_monitor().map_err(|e| e.to_string())?.ok_or("No monitor")?;
                    let area = work.work_area();
                    let requested = requested.unwrap_or(Point {
                        x: f64::from(area.position.x) + f64::from(area.size.width) * 0.6,
                        y: f64::from(area.position.y) + f64::from(area.size.height) * 0.6,
                    });
                    let render = RenderConfig { canvas_width: 128, canvas_height: 112, scale: 1.0, anchor_x: 0.5, anchor_y: 0.82 };
                    let placed = window::place(&pet, &render, 1.0, Some(requested))?;
                    let immediate = window::anchor(&pet, &render)?;
                    Ok::<_, String>((pet, render, placed, immediate))
                })();
                let _ = tx.send(result);
            }).map_err(|e| e.to_string())?;
            let (pet, render, placed, immediate) = rx.recv_timeout(Duration::from_secs(5)).map_err(|e| e.to_string())??;
            if let Some(previous) = saved {
                if placed != previous { return Err(format!("Restart changed saved anchor: {previous:?} -> {placed:?}")); }
            }
            Settings { last_position: Some(placed), ..Settings::default() }.save(&path)?;
            saved = Settings::load(&path).last_position;
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                let actual = window::anchor(&pet, &render)?;
                // AppKit may align a half logical pixel to a whole pixel on Retina.
                if (actual.x - placed.x).abs() <= 1.0 && (actual.y - placed.y).abs() <= 1.0 {
                    println!("Restart {restart}: saved={placed:?}, immediate={immediate:?}, restored={actual:?}");
                    if restart > 0 && Some(actual) != saved { return Err(format!("Restart drifted: saved={saved:?}, actual={actual:?}")); }
                    // Match the normal drag-end / exit save, using settled geometry.
                    Settings { last_position: Some(actual), ..Settings::default() }.save(&path)?;
                    saved = Settings::load(&path).last_position;
                    break;
                }
                if Instant::now() >= deadline { return Err(format!("Position did not restore: saved={placed:?}, actual={actual:?}")); }
                std::thread::sleep(Duration::from_millis(10));
            }
            pet.close().map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(path);
    result
}

fn main() {
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let code = tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let result = check(&handle);
                if let Err(error) = &result { eprintln!("Position restore failed: {error}"); }
                handle.exit(if result.is_ok() { 0 } else { 1 });
            });
            Ok(())
        })
        .build(context).expect("build test app")
        .run_return(|_, event| {
            if let tauri::RunEvent::ExitRequested { code: None, api, .. } = event { api.prevent_exit(); }
        });
    std::process::exit(code);
}
