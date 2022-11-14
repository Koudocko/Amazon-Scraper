#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

#[macro_use(lazy_static)]
extern crate lazy_static;

use scraper::{Html, Selector};
use calamine::{Range, Xlsx, open_workbook, Reader, DataType};
use std::sync::Mutex;
use tauri::api::dialog;
use tauri::{CustomMenuItem, Menu};
use std::thread;

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
fn scrape_data(body: &str)-> Result<[String; 3], ()>{
    let mut product: [String; 3] = Default::default(); 

    // Scrape html for data
    let fragment = Html::parse_document(&body);
    if let Some(name) = fragment.select(
        &Selector::parse(r#"span[id="productTitle"]"#).unwrap())
        .next(){
        product[0] = name.inner_html().trim().to_owned();
    }

    if let Some(image) = fragment.select(
        &Selector::parse(r#"img[id="imgBlkFront"]"#).unwrap())
        .next(){
        product[1] = image.value().attr("src").unwrap().to_owned();
    }
    else if let Some(image) = fragment.select(
        &Selector::parse(r#"img[id="landingImage"]"#).unwrap())
        .next(){
        product[1] = image.value().attr("src").unwrap().to_owned();
    }

    if product[0].is_empty(){
        Err(())
    }
    else{
        Ok(product)
    }
}

#[tauri::command]
async fn get_product(lpn: String)-> Option<[String; 3]>{
    if DATABASE.lock().unwrap().is_some(){
        if let Ok(asin) = lookup_product(&lpn){
            if let Ok(body) = reqwest::get(format!("https://amazon.com/dp/{}", asin))
                .await.unwrap().text().await{
                if let Ok(data) = scrape_data(&body){
                    return Some(data);
                }
            }
        }
    }

    None
}

#[tokio::main]
async fn main(){
    let import_spreadsheet = CustomMenuItem::new("import_spreadsheet".to_string(), "Import Spreadsheet");
    let menu = Menu::new()
        .add_item(import_spreadsheet);

    tauri::Builder::default()
        .menu(menu)
        .on_menu_event(|event|{
            match event.menu_item_id(){
                "import_spreadsheet" =>{
                    dialog::FileDialogBuilder::default()
                        .add_filter("", &["xlsx"])
                        .pick_file(|path_buf|{
                            if let Some(path) = path_buf{
                                if DATABASE.lock().unwrap().is_none(){
                                    // Load excel database
                                    thread::spawn(||{
                                        let mut document: Xlsx<_> = open_workbook(path).unwrap();

                                        if let Some(Ok(sheet)) = document.worksheet_range_at(0){
                                            *DATABASE.lock().unwrap() = Some(sheet);
                                        }
                                    });
                                }
                            }
                        })
                }
                _ =>{}
            }
        })
        .invoke_handler(tauri::generate_handler![get_product])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
