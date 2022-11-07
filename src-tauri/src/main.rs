#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

#[macro_use(lazy_static)]
extern crate lazy_static;

use calamine::{Range, Xlsx, open_workbook, Reader, DataType};
use std::sync::Mutex;

lazy_static!{
    static ref DATABASE: Mutex<Option<Range<DataType>>> = Mutex::new(None);
}

fn main() {
    if DATABASE.lock().unwrap().is_none(){
        // Load excel database
        let path = format!("{}/../database.xlsx", env!("CARGO_MANIFEST_DIR"));
        let mut document: Xlsx<_> = open_workbook(path).unwrap();

        if let Some(Ok(sheet)) = document.worksheet_range("F2665-YYZ9_YYY000hp90s"){
            *DATABASE.lock().unwrap() = Some(sheet);
        }
    }

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
