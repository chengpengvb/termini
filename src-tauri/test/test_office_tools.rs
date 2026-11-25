use std::path::PathBuf;
use docx_rust::DocxFile;
use rust_xlsxwriter::Workbook;
use calamine::{open_workbook_auto, Data, Reader};
use crate::module::ai::tools::common::tool_pdf_read::ToolPdfRead;

const PDF_DIR: &str = r"C:\project\app\dskb\支付对接";
const DOCX_DIR: &str = r"C:\Users\chengpeng\Desktop\start";
const AGG_PDF: &str = r"C:\project\app\dskb\支付对接\汇聚支付聚合支付API接口文档V2.6.10.pdf";

fn collect_files(dir: &str, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(dir).unwrap_or_else(|error| panic!("read_dir {} failed: {}", dir, error));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|value| value.eq_ignore_ascii_case(ext)) {
            out.push(path);
        }
    }
    out
}

#[test]
fn probe_pdf_directory() {
    if std::env::var("TERMINI_DATA_DIR").is_err() {
        let temp = std::env::temp_dir().join("termix-office-tools");
        std::fs::create_dir_all(&temp).unwrap();
        std::env::set_var("TERMINI_DATA_DIR", &temp);
    }
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let dll_path = match runtime.block_on(ToolPdfRead::ensure_pdfium_dll()) {
        Ok(path) => path,
        Err(error) => {
            println!("[pdf] skip: pdfium engine unavailable => {}", error.detail());
            return;
        }
    };
    let files = collect_files(PDF_DIR, "pdf");
    assert!(!files.is_empty(), "no pdf file under {}", PDF_DIR);
    println!("[pdf] scan dir={} files={} dll={}", PDF_DIR, files.len(), dll_path.display());
    let mut ok = 0usize;
    let mut err = 0usize;
    for file in &files {
        let name = file.file_name().and_then(|value| value.to_str()).unwrap_or("<invalid>").to_string();
        match ToolPdfRead::extract_text_via_pdfium(&dll_path, file, 0, 50) {
            Ok(extracted) => {
                ok += 1;
                let chars = extracted.text.chars().count();
                let preview: String = extracted.text.chars().take(80).collect();
                println!("[pdf ok] {} pages={}/{} chars={} preview={:?}", name, extracted.taken, extracted.total, chars, preview);
            }
            Err(error) => {
                err += 1;
                println!("[pdf err] {} => {}", name, error.detail());
            }
        }
    }
    println!("[pdf] summary ok={} err={}", ok, err);
}

fn ensure_pdfium_for_test() -> Option<PathBuf> {
    if std::env::var("TERMINI_DATA_DIR").is_err() {
        let temp = std::env::temp_dir().join("termix-office-tools");
        std::fs::create_dir_all(&temp).unwrap();
        std::env::set_var("TERMINI_DATA_DIR", &temp);
    }
    let runtime = tokio::runtime::Runtime::new().unwrap();
    match runtime.block_on(ToolPdfRead::ensure_pdfium_dll()) {
        Ok(path) => Some(path),
        Err(error) => {
            println!("[pdf] skip: pdfium engine unavailable => {}", error.detail());
            None
        }
    }
}

#[test]
fn probe_pdf_serial_single_pages() {
    let Some(dll_path) = ensure_pdfium_for_test() else {
        return;
    };
    let path = PathBuf::from(AGG_PDF);
    let mut ok = 0usize;
    let mut blank = Vec::new();
    let mut failed = Vec::new();
    for index in 50..92usize {
        match ToolPdfRead::extract_text_via_pdfium(&dll_path, &path, index, 1) {
            Ok(extracted) => {
                let body = extracted.text.chars().count().saturating_sub(format!("--- page {} ---\n\n", index + 1).chars().count());
                if body == 0 {
                    blank.push(index + 1);
                } else {
                    ok += 1;
                }
            }
            Err(error) => {
                println!("[serial err] page={} => {}", index + 1, error.detail());
                failed.push(index + 1);
            }
        }
    }
    println!("[serial] pages=50..92 ok={} blank={:?} failed={:?}", ok, blank, failed);
    assert!(failed.is_empty(), "serial single-page read failed on pages {:?}", failed);
}

#[test]
fn probe_pdf_concurrent_read() {
    let Some(dll_path) = ensure_pdfium_for_test() else {
        return;
    };
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build().unwrap();
    let results = runtime.block_on(async {
        let mut handles = Vec::new();
        for index in 0..8usize {
            let dll = dll_path.clone();
            handles.push(tokio::task::spawn_blocking(move || {
                let path = PathBuf::from(AGG_PDF);
                let start = index * 10;
                match ToolPdfRead::extract_text_via_pdfium(&dll, &path, start, 5) {
                    Ok(extracted) => Ok((start, extracted.taken, extracted.text.chars().count())),
                    Err(error) => Err((start, error.detail().to_string())),
                }
            }));
        }
        let mut out = Vec::new();
        for handle in handles {
            out.push(handle.await.unwrap());
        }
        out
    });
    let mut ok = 0usize;
    let mut err = 0usize;
    for item in &results {
        match item {
            Ok((start, taken, chars)) => {
                ok += 1;
                println!("[conc ok] start_page={} taken={} chars={}", start + 1, taken, chars);
            }
            Err((start, detail)) => {
                err += 1;
                println!("[conc err] start_page={} => {}", start + 1, detail);
            }
        }
    }
    println!("[conc] tasks=8 ok={} err={}", ok, err);
}

#[test]
fn probe_docx_directory() {
    let files = collect_files(DOCX_DIR, "docx");
    assert!(!files.is_empty(), "no docx file under {}", DOCX_DIR);
    println!("[docx] scan dir={} files={}", DOCX_DIR, files.len());
    let mut ok = 0usize;
    let mut err = 0usize;
    for file in &files {
        let name = file.file_name().and_then(|value| value.to_str()).unwrap_or("<invalid>").to_string();
        match DocxFile::from_file(file).and_then(|handle| {
            let docx = handle.parse()?;
            let blocks = docx.document.body.content.len();
            let text = docx.document.body.text();
            Ok((blocks, text))
        }) {
            Ok((blocks, text)) => {
                ok += 1;
                let chars = text.chars().count();
                let preview: String = text.chars().take(80).collect();
                println!("[docx ok] {} blocks={} chars={} preview={:?}", name, blocks, chars, preview);
            }
            Err(error) => {
                err += 1;
                println!("[docx err] {} => {}", name, error);
            }
        }
    }
    println!("[docx] summary ok={} err={}", ok, err);
}

#[test]
fn round_trip_xlsx() {
    let dir = std::env::temp_dir().join("termix-office-tools");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("round_trip.xlsx");

    let mut wb = Workbook::new();
    let orders = wb.add_worksheet();
    orders.set_name("orders").unwrap();
    orders.write_string(0, 0, "id").unwrap();
    orders.write_string(0, 1, "amount").unwrap();
    orders.write_string(0, 2, "paid").unwrap();
    orders.write_number(1, 0, 1.0).unwrap();
    orders.write_number(1, 1, 88.5).unwrap();
    orders.write_boolean(1, 2, true).unwrap();
    orders.write_number(2, 0, 2.0).unwrap();
    orders.write_number(2, 1, 12.0).unwrap();
    orders.write_boolean(2, 2, false).unwrap();
    let summary = wb.add_worksheet();
    summary.set_name("summary").unwrap();
    summary.write_string(0, 0, "note").unwrap();
    summary.write_string(0, 1, "value").unwrap();
    summary.write_string(1, 0, "订单总数").unwrap();
    summary.write_number(1, 1, 2.0).unwrap();
    wb.save(&path).unwrap();
    println!("[xlsx] wrote {}", path.display());

    let mut book = open_workbook_auto(&path).unwrap();
    let sheet_names = book.sheet_names().to_owned();
    assert_eq!(sheet_names, vec!["orders".to_string(), "summary".to_string()]);
    let orders_range = book.worksheet_range("orders").unwrap();
    assert_eq!(orders_range.get_size(), (3, 3));
    let orders_rows: Vec<Vec<String>> = orders_range.rows().map(|row| row.iter().map(cell_to_string).collect()).collect();
    println!("[xlsx orders] {:?}", orders_rows);
    assert_eq!(orders_rows[0], vec!["id", "amount", "paid"]);
    assert_eq!(orders_rows[1], vec!["1", "88.5", "true"]);
    assert_eq!(orders_rows[2], vec!["2", "12", "false"]);
    let summary_range = book.worksheet_range("summary").unwrap();
    let summary_rows: Vec<Vec<String>> = summary_range.rows().map(|row| row.iter().map(cell_to_string).collect()).collect();
    println!("[xlsx summary] {:?}", summary_rows);
    assert_eq!(summary_rows[1], vec!["订单总数", "2"]);
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(text) => text.clone(),
        Data::Float(value) => {
            if value.fract() == 0.0 {
                (*value as i64).to_string()
            } else {
                value.to_string()
            }
        }
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        Data::DateTime(value) => value.to_string(),
        Data::DateTimeIso(value) => value.clone(),
        Data::DurationIso(value) => value.clone(),
        Data::Error(error) => format!("#ERROR:{:?}", error),
    }
}
