//! Offscreen Korean interface smoke check; no desktop interaction is required.
use std::sync::atomic::{AtomicBool, Ordering};
use deckcraft_engine::Session;
use deckcraft_ui_egui::{Services, SlideApp, i18n::Language};

static READY: AtomicBool = AtomicBool::new(false);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or_else(|| std::io::Error::other("provide output PNG path"))?;
    let mut session = Session::new();
    deckcraft_engine::sample::open_sample(&mut session).map_err(std::io::Error::other)?;
    let mut app = SlideApp::new(session, Services::default());
    app.integrated_titlebar = true;
    app.run("app.language", serde_json::json!({"language": "ko"})).map_err(std::io::Error::other)?;
    app.run("shape.insert", serde_json::json!({"preset": "rect", "rect": [80, 420, 740, 90], "text": "한국어 슬라이드 테스트: 안녕하세요. 가나다라마."})).map_err(std::io::Error::other)?;
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(1_000_000)
        .wgpu()
        .build_ui_state(|ui, app: &mut SlideApp| {
            if READY.load(Ordering::Relaxed) {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            }
        }, app);
    harness.input_mut().max_texture_side = Some(8192);
    READY.store(true, Ordering::Relaxed);
    for _ in 0..180 {
        let mut raw = std::mem::take(harness.input_mut());
        harness.state_mut().raw_input_hook(&mut raw);
        *harness.input_mut() = raw;
        harness.step();
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    if harness.state().ui.language != Language::Ko {
        return Err(std::io::Error::other("Korean language selection was not retained").into());
    }
    harness.render()?.save(&path)?;
    println!("Korean interface rendered: {path}");
    Ok(())
}
