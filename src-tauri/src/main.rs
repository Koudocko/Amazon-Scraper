#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use calamine::{Range, Xlsx, open_workbook, Reader, DataType};
use tauri::{CustomMenuItem, Menu, Submenu, Window, Manager};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use scraper::{Html, Selector};
use std::sync::{Mutex, mpsc};
use std::fs::OpenOptions;
use tauri::api::dialog;
use std::path::Path;
use std::io::Write;
use std::io::Read;
use std::thread;

static BROKEN_ENTRIES: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
static DATABASE: Mutex<Vec<Range<DataType>>> = Mutex::new(Vec::new());
static mut OUTPUT_PATH: Option<String> = None;
static mut WINDOW: Option<Window> = None;

fn lookup_product(lpn: &str)-> Result<String, ()>{
    let sheets = (*DATABASE.lock().unwrap()).clone();
    for sheet in sheets{
        let lpn_idx = (0..sheet.width()).find(|idx| sheet.get((0, *idx)).unwrap() == "LPN").unwrap();
        let asin_idx = (0..sheet.width()).find(|idx| sheet.get((0, *idx)).unwrap() == "Asin").unwrap();

        for row in sheet.rows(){
             let curr = row[lpn_idx].to_string();
             if curr == lpn{
                 return Ok(row[asin_idx].to_string());
             }
        }
    }

    Err(())
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
async fn find_product(name: String)-> Vec<[String; 3]>{
    let mut matches: Vec<(String, i32)> = Vec::new();

    {
        let (mut broken_guard, db_guard) = (BROKEN_ENTRIES.lock().unwrap(), DATABASE.lock().unwrap());
        if broken_guard.is_empty() && !db_guard.is_empty(){
            for sheet in db_guard.clone(){
                let lpn_idx = (0..sheet.width()).find(|idx| sheet.get((0, *idx)).unwrap() == "LPN").unwrap();
                let name_idx = (0..sheet.width()).find(|idx| sheet.get((0, *idx)).unwrap() == "ItemDesc").unwrap();
                let asin_idx = (0..sheet.width()).find(|idx| sheet.get((0, *idx)).unwrap() == "Asin").unwrap();

                for row in sheet.rows(){
                    let curr = row[lpn_idx].to_string();
                    if curr.is_empty(){
                        broken_guard.push(
                             (row[name_idx].to_string(),
                             row[asin_idx].to_string())
                        );
                    }
                }
            }
        }

        let matcher = SkimMatcherV2::default();
        
        for broken_entry in broken_guard.clone(){
            let mut idx = 0;

            if let Some(val) = matcher.fuzzy_match(&broken_entry.0, &name){
                if val >= 100{
                    for ele in &matches{
                        if val < ele.1 as i64{
                            idx += 1;
                        }
                    }

                    matches.insert(idx, (broken_entry.1.clone(), val as i32));
                }
            }
        }
    }

    let mut tasks = Vec::new();
    
    for request in matches{
        tasks.push((request.0.to_owned(), tokio::spawn(reqwest::get(format!("https://amazon.com/dp/{}", request.0)))));
    }

    let mut found_list = Vec::<[String; 3]>::new();
    for task in tasks{
        if let Ok(body) = task.1.await.unwrap().unwrap().text().await{
            if let Ok(data) = scrape_data(&body){
                let mut element: [String; 3] = Default::default();
                element[0] = data[0].to_owned();
                element[1] = data[1].to_owned();
                element[2] = task.0;
                found_list.push(element);
            }
        }
    }

    found_list
}

#[tauri::command]
async fn get_product(lpn: String)-> Option<[String; 4]>{
    if !DATABASE.lock().unwrap().is_empty(){
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

#[tauri::command]
async fn write_product(information: [String; 10])-> Option<bool>{
    unsafe{
        if OUTPUT_PATH.is_some() && !Path::new(&OUTPUT_PATH.clone().unwrap()).exists(){
            OUTPUT_PATH = None;
        }

        let (tx, rx) = mpsc::channel();
        if OUTPUT_PATH.is_none(){
            dialog::FileDialogBuilder::default()
            .add_filter("", &["csv"])
            .pick_file(move |path_buf|{
                if let Some(path) = path_buf{
                    OUTPUT_PATH = Some(path.into_os_string().into_string().unwrap());
                }
                tx.send(true).unwrap();
            })
        }
        else{
            tx.send(true).unwrap();
        }

        rx.recv().unwrap();

        if let Some(path) = OUTPUT_PATH.clone(){
            let mut handle = OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .unwrap();

            let mut buf = String::new();
            handle.read_to_string(&mut buf).unwrap();

            if !buf.contains('\n'){
                let header = String::from("Lot,Lead,Description,Condition,Vendor,Shipping,Min Bid,Category,MSRP\n");
                handle.write_all(header.as_bytes()).unwrap();
            }

            for field in 0..9{
                handle.write_all(("\"".to_owned() + &information[field] + "\",").as_bytes()).unwrap();
            }

            handle.write_all("\n".as_bytes()).unwrap();

            WINDOW.clone().unwrap().eval(r#"
                var div = document.getElementById("outputState");
                div.style.color = 'var(--good)';
                div.innerHTML = "Loaded.";
            "#).unwrap();

            if let Ok(img) = reqwest::get(&information[9]).await{
                let img = img.bytes().await.unwrap();
                let img = image::load_from_memory(&img);

                let path = OUTPUT_PATH.clone().unwrap();
                let parent_path = std::path::Path::new(&path)
                    .parent().unwrap();
                img.unwrap().save_with_format(
                    parent_path.join("LOT".to_owned() + &information[0] + ".jpg"), 
                    image::ImageFormat::Jpeg
                ).unwrap();
            }

            Some(true)
        }
        else{
            WINDOW.clone().unwrap().eval(r#"
                var div = document.getElementById("outputState");
                div.style.color = 'var(--bad)';
                div.innerHTML = "Not Loaded...";
            "#).unwrap();
            None
        }
    }
}

#[tokio::main]
async fn main(){
    let spreadsheet = CustomMenuItem::new("input".to_string(), "Input Spreadsheet");
    let csv = CustomMenuItem::new("output".to_string(), "Output Spreadsheet");
    let submenu = Submenu::new("Import", Menu::new().add_item(spreadsheet).add_item(csv));

    let menu = Menu::new()
        .add_submenu(submenu);

    tauri::Builder::default()
    .menu(menu)
    .setup(|app|{
        unsafe{ WINDOW = Some(app.get_window("main").unwrap()); }
        Ok(())
    })
    .on_menu_event(|event|{
        match event.menu_item_id(){
            "input" =>{
                dialog::FileDialogBuilder::default()
                .add_filter("", &["xlsx"])
                .pick_file(move |path_buf|{
                    if let Some(path) = path_buf{
                        event.window().eval(r#"
                            var div = document.getElementById("inputState");
                            div.style.color = 'var(--warning)';
                            div.innerHTML = "Loading...";
                        "#).unwrap();
                        thread::spawn(move ||{
                            let mut document: Xlsx<_> = open_workbook(path).unwrap();

                            if let Some(Ok(sheet)) = document.worksheet_range_at(0){
                                DATABASE.lock().unwrap().push(sheet);
                                event.window().eval(r#"
                                    div.style.color = 'var(--good)';
                                    div.innerHTML = "Loaded.";
                                "#).unwrap();
                            }
                            else{
                                event.window().eval(r#"
                                    div.style.color = 'var(--bad)';
                                    div.innerHTML = "Not Loaded!";
                                "#).unwrap();
                            }
                        });
                    }
                })
            }
            "output" =>{
                dialog::FileDialogBuilder::default()
                .add_filter("", &["csv"])
                .pick_file(move |path_buf|{
                    if let Some(path) = path_buf{
                        event.window().eval(r#"
                            var div = document.getElementById("outputState");
                            div.style.color = 'var(--warning)';
                            div.innerHTML = "Loading...";
                        "#).unwrap();
                        unsafe{
                            OUTPUT_PATH = Some(path.into_os_string().into_string().unwrap());
                            event.window().eval(r#"
                                div.style.color = 'var(--good)';
                                div.innerHTML = "Loaded.";
                            "#).unwrap();
                        }
                    }
                })
            }
            _ =>{}
        }
    })
    .invoke_handler(tauri::generate_handler![get_product, write_product, find_product])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
