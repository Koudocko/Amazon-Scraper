#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use calamine::{Range, Xlsx, open_workbook, Reader, DataType};
use tauri::{CustomMenuItem, Menu, Submenu, Window, Manager};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use scraper::{Html, Selector};
use std::collections::{HashSet, HashMap};
use std::sync::{Mutex, mpsc};
use once_cell::sync::Lazy;
use tauri::api::dialog;
use std::path::Path;
use std::thread;

static BROKEN_ENTRIES: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
static DATABASE: Mutex<Vec<Range<DataType>>> = Mutex::new(Vec::new());
static mut OUTPUT_PATH: Option<String> = None;
static mut WINDOW: Option<Window> = None;
static mut INPUT_COUNT: i32 = 0;
static INPUT_STATES: Mutex<(String, String)> = Mutex::new((String::new(), String::new()));
static SEARCH_RESULTS: Lazy<Mutex<HashMap<String, Vec<String>>>> = Lazy::new(||{
    Mutex::new(HashMap::new())
});

fn get_idx(sheet: &Range<DataType>, pattern: &str)-> Option<usize>{
    (0..sheet.width()).find(|idx| sheet.get((0, *idx)).unwrap() == pattern)
}

fn lookup_product(search: &str, key: &str)-> Result<String, ()>{
    let sheets = (*DATABASE.lock().unwrap()).clone();
    for sheet in sheets{
        if let Some(key_idx) = get_idx(&sheet, search){
            if let Some(asin_idx) = get_idx(&sheet, "Asin"){
                for row in sheet.rows(){
                     let curr = row[key_idx].to_string();
                     if curr == key{
                         return Ok(row[asin_idx].to_string());
                     }
                }
            }
        }
    }

    Err(())
}

fn scrape_data(body: &str)-> Result<Vec<String>, ()>{
    let mut product = vec![String::new(); 4];

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
async fn find_product(name: String)-> Vec<Vec<String>>{
    let mut matches: Vec<(String, i32)> = Vec::new();
    let mut duplicates = HashSet::new();

    {
        let (mut broken_guard, db_guard) = (BROKEN_ENTRIES.lock().unwrap(), DATABASE.lock().unwrap());
        if broken_guard.is_empty() && !db_guard.is_empty(){
            for sheet in db_guard.clone(){
                if let Some(lpn_idx) = get_idx(&sheet, "LPN"){
                    if let Some(name_idx) = get_idx(&sheet, "ItemDesc"){
                        if let Some(asin_idx) = get_idx(&sheet, "Asin"){
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
                }
            }
        }

        let matcher = SkimMatcherV2::default();
        
        for broken_entry in broken_guard.clone(){
            let mut idx = 0;

            if let Some(val) = matcher.fuzzy_match(&broken_entry.0, &name){
                if val >= 100 && duplicates.get(&val).is_none(){
                    duplicates.insert(val);

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
    let mut found_list = Vec::<Vec<String>>::new();
    
    let client = reqwest::Client::builder()
        .gzip(true)
        .build().unwrap();

    for request in matches{
        println!("found");
        if let Some(found) = SEARCH_RESULTS.lock().unwrap().get(&request.0){
            found_list.push(found.to_owned());
        }
        else{
            tasks.push((request.0.to_owned(), tokio::spawn(client.get(format!("https://amazon.com/dp/{}", request.0)).send())));
        }
    }

    for task in tasks{
        if let Ok(body) = task.1.await.unwrap().unwrap().text().await{
            if let Ok(data) = scrape_data(&body){
                let mut element = Vec::new();
                element.extend(data);
                element.push(task.0.to_owned());
                found_list.push(element.to_owned());
                SEARCH_RESULTS.lock().unwrap().insert(task.0, element);
            }
        }
    }

    println!("{}", found_list.len());
    found_list
}

#[tauri::command]
fn get_result(key: String)-> Option<Vec<String>>{
    SEARCH_RESULTS.lock().unwrap().get(&key).cloned()
}

#[tauri::command]
async fn get_product(search: String, key: String)-> Option<Vec<String>>{
    if !DATABASE.lock().unwrap().is_empty(){
        if let Ok(asin) = lookup_product(&search, &key){
            let client = reqwest::Client::builder()
                .gzip(true)
                .build().unwrap();

            if let Ok(body) = client.get(format!("https://amazon.com/dp/{}", asin)).send()
                .await.unwrap().text().await{
                if let Ok(mut data) = scrape_data(&body){
                    data.extend(vec![asin]);
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
            WINDOW.clone().unwrap().eval(r#"
                var div = document.getElementById("outputState");
                div.style.color = 'var(--good)';
                div.innerHTML = "Loaded.";
            "#).unwrap();

            let mut handle_read = csv::ReaderBuilder::new()
                .has_headers(false)
                .from_path(&path).unwrap();

            let records = handle_read.records().skip(1).map(|record|{
                if let Ok(record) = record
                    {record}
                else
                    {csv::StringRecord::new()}
            }).collect::<Vec<csv::StringRecord>>();

            let mut handle_write = csv::Writer::from_path(&path).unwrap();
            handle_write.write_record(&["Lot","Lead","Description","Condition","Vendor","Shipping","Min Bid","Category","MSRP"]).unwrap();

            for record in &records{
                handle_write.write_byte_record(record.as_byte_record()).unwrap();
            }

            if !records.iter().any(|record|{
                record[0] == information[0]
            }){
                handle_write.write_record(&information[0..9]).unwrap();

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
                None        
            }
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

#[tauri::command]
fn on_load()-> (String, String){
    (*INPUT_STATES.lock().unwrap()).to_owned()
}

#[tauri::command]
fn on_leave(input: String, output: String){
    (*INPUT_STATES.lock().unwrap()).0 = input;
    (*INPUT_STATES.lock().unwrap()).1 = output;
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
                        event.window().eval(&(r#"
                            var div = document.getElementById("inputStates");
                            div.innerHTML += ""#.to_owned() 
                            + path.file_name().unwrap().to_str().unwrap()
                            + " | " + r#"<span id='inputState"#  + &unsafe{INPUT_COUNT}.to_string()
                            + r#"' style='color: var(--warning);'>Loading...</span><br>";"#
                        )).unwrap();
                        thread::spawn(move ||{
                            let idx = unsafe{INPUT_COUNT};
                            let mut document: Xlsx<_> = open_workbook(path).unwrap();

                            if let Some(Ok(sheet)) = document.worksheet_range_at(0){
                                DATABASE.lock().unwrap().push(sheet);
                                event.window().eval(&(r#"
                                    var subDiv = document.getElementById("inputState"#.to_owned()
                                    + &idx.to_string() + r#"");
                                    subDiv.style.color = 'var(--good)';
                                    subDiv.innerHTML = "Loaded.""#
                                )).unwrap();
                                unsafe{INPUT_COUNT += 1;}
                            }
                            else{
                                event.window().eval(&(r#"
                                    var subDiv = document.getElementById("inputState"#.to_owned()
                                    + &idx.to_string() + r#"");
                                    subDiv.style.color = 'var(--bad)';
                                    subDiv.innerHTML = "Loading Failed!""#
                                )).unwrap();
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
    .invoke_handler(tauri::generate_handler![get_product, write_product, find_product, on_load, on_leave, get_result])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
