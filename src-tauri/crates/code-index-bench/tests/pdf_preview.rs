use std::path::PathBuf;
use pdfium_render::prelude::*;

const PREVIEW_WIDTH: i32 = 1240;

fn pdfium_dll() -> Option<PathBuf> {
    let data_dir = std::env::var("TERMINI_DATA_DIR").map(PathBuf::from).or_else(|_| std::env::var("APPDATA").map(|appdata| PathBuf::from(appdata).join("com.termini.www"))).ok()?;
    let dll = data_dir.join("pdfium").join(if cfg!(target_os = "windows") {
        "pdfium.dll"
    } else if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else {
        "libpdfium.so"
    });
    dll.is_file().then_some(dll)
}

#[test]
fn render_pdf_pages_to_png() {
    let Some(source) = std::env::var("TERMINI_PDF_PREVIEW").ok().map(PathBuf::from).filter(|path| path.is_file()) else {
        eprintln!("skip: 需要 TERMINI_PDF_PREVIEW 指向一个 pdf 文件");
        return;
    };
    let Some(dll) = pdfium_dll() else {
        eprintln!("skip: 找不到 pdfium 动态库");
        return;
    };
    let pdfium = Pdfium::new(Pdfium::bind_to_library(&dll).expect("bind pdfium"));
    let document = pdfium.load_pdf_from_file(&source, None).expect("load pdf");
    let count = document.pages().len() as i32;
    eprintln!("pages={}", count);
    let stem = source.file_stem().and_then(|value| value.to_str()).unwrap_or("preview").to_string();
    for index in [0, count - 1] {
        let page = document.pages().get(index).expect("page");
        let text = page.text().expect("text").all();
        eprintln!("page {} text head: {}", index + 1, text.chars().take(80).collect::<String>().replace('\n', " | "));
        std::fs::write(std::env::temp_dir().join(format!("{}_page{}.txt", stem, index + 1)), &text).expect("save text");
        let preview = std::env::temp_dir().join(format!("{}_page{}.png", stem, index + 1));
        let bitmap = page.render_with_config(&PdfRenderConfig::new().set_target_width(PREVIEW_WIDTH)).expect("render");
        bitmap.as_image().expect("image").save(&preview).expect("save png");
        eprintln!("preview: {}", preview.display());
    }
}
