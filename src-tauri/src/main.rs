#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use scraper::{Html, Selector};
use calamine::{Range, Xlsx, open_workbook, Reader, DataType};
use std::io::Read;
use std::sync::Mutex;
use tauri::api::dialog;
use tauri::{CustomMenuItem, Menu, Submenu};
use std::thread;
use std::fs::OpenOptions;
use std::io::Write;

static mut DATABASE: Mutex<Option<Range<DataType>>> = Mutex::new(None);
static mut OUTPUT_PATH: Mutex<Option<String>> = Mutex::new(None);

fn lookup_product(lpn: &str)-> Result<String, ()>{
    unsafe{
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
}

fn scrape_data(body: &str)-> Result<[String; 4], ()>{
    let mut product: [String; 4] = Default::default(); 

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

    for description in fragment.select(
        &Selector::parse(r#"div[id="feature-bullets"] > ul > li > span.a-list-item"#).unwrap()){
        product[2] += &(description.inner_html().trim().to_owned() + " ; ");
    }
    if let Some(description) = fragment.select(
        &Selector::parse(r#"div[id="bookDescription_feature_div"] > div > div > span"#).unwrap())
        .next(){
        product[2] += &("\n".to_owned() + description.inner_html().trim());
    }

    if let Some(msrp) = fragment.select(
        &Selector::parse(r#"span > span.a-offscreen"#).unwrap())
        .next(){
        product[3] = msrp.inner_html().trim().to_owned();
        
    }

    if product[0].is_empty(){
        Err(())
    }
    else{
        Ok(product)
    }
}

#[tauri::command]
async fn get_product(lpn: String)-> Option<[String; 4]>{
    unsafe{
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
}

#[tauri::command]
async fn write_product(information: [String; 9])-> Option<()>{
    unsafe{
        if (*OUTPUT_PATH.lock().unwrap()).is_none(){
            dialog::FileDialogBuilder::default()
            .add_filter("", &["csv"])
            .pick_file(|path_buf|{
                if let Some(path) = path_buf{
                    *OUTPUT_PATH.lock().unwrap() = Some(path.into_os_string().into_string().unwrap());
                }
            })
        }

        while (*OUTPUT_PATH.lock().unwrap()).is_none(){}
        let mut handle = OpenOptions::new()
            .read(true)
            .write(true)
            .open((*OUTPUT_PATH.lock().unwrap()).clone().unwrap())
            .unwrap();

        let mut buf = String::new();
        handle.read_to_string(&mut buf).unwrap();

        if !buf.contains('\n'){
            let header = String::from("Lot,Lead,Description 1,Description 2/Condition,Vendor,Shipping,Min Bid,Category,MSRP\n");
            handle.write_all(header.as_bytes()).unwrap();
        }

        for field in information{
            handle.write_all((field + ",").as_bytes()).unwrap();
        }

        handle.write_all("\n".as_bytes()).unwrap();

        Some(())
    }
}

#[tokio::main]
async fn main(){
    let spreadsheet = CustomMenuItem::new("spreadsheet".to_string(), "Spreadsheet");
    let submenu = Submenu::new("Import", Menu::new().add_item(spreadsheet));
    let menu = Menu::new()
        .add_submenu(submenu);

    tauri::Builder::default()
    .menu(menu)
    .on_menu_event(|event|{
        match event.menu_item_id(){
            "spreadsheet" =>{
                dialog::FileDialogBuilder::default()
                .add_filter("", &["xlsx"])
                .pick_file(|path_buf|{
                    if let Some(path) = path_buf{
                        unsafe{
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
                    }
                })
            }
            _ =>{}
        }
    })
    .invoke_handler(tauri::generate_handler![get_product, write_product])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
