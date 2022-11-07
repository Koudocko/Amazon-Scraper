#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

#[macro_use(lazy_static)]
extern crate lazy_static;

use scraper::{Html, Selector};
use calamine::{Range, Xlsx, open_workbook, Reader, DataType};
use std::sync::Mutex;

lazy_static!{
    static ref DATABASE: Mutex<Option<Range<DataType>>> = Mutex::new(None);
}

fn lookup_product(lpn: &str)-> Result<String, ()>{
    let mut asin = String::new();

    let sheet = (*DATABASE.lock().unwrap()).clone().unwrap();
    let lpn_idx = (0..sheet.width()).find(|idx| sheet.get((0, *idx)).unwrap() == "LPN").unwrap();
    let asin_idx = (0..sheet.width()).find(|idx| sheet.get((0, *idx)).unwrap() == "Asin").unwrap();

    for row in sheet.rows(){
         let curr = row[lpn_idx].to_string();
         if curr == lpn{
             asin = row[asin_idx].to_string();
             break;
         }
    }

    if asin.is_empty(){
        Err(())
    }
    else{
        Ok(asin)
    }
}
fn scrape_data(body: &str)-> Result<(String, String), ()>{
    let mut product = (String::new(), String::new());

    // Scrape html for data
    let fragment = Html::parse_document(&body);
    if let Some(name) = fragment.select(
        &Selector::parse(r#"span[id="productTitle"]"#).unwrap())
        .next(){
        product.0 = name.inner_html().trim().to_owned();
    }

    if let Some(image) = fragment.select(
        &Selector::parse(r#"img[id="imgBlkFront"]"#).unwrap())
        .next(){
        product.1 = image.value().attr("src").unwrap().to_owned();
    }
    else if let Some(image) = fragment.select(
        &Selector::parse(r#"img[id="landingImage"]"#).unwrap())
        .next(){
        product.1 = image.value().attr("src").unwrap().to_owned();
    }

    if product.0.is_empty(){
        Err(())
    }
    else{
        Ok(product)
    }
}

#[tauri::command]
async fn get_product(lpn: String)-> (String, String){
    if let Ok(asin) = lookup_product(&lpn){
        if let Ok(body) = reqwest::get(format!("https://amazon.com/dp/{}", asin))
            .await.unwrap().text().await{
            if let Ok(data) = scrape_data(&body){
                return data;
            }
        }
    }

    (String::from("No Results Found"), String::new())
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
        .invoke_handler(tauri::generate_handler![get_product])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
