#![allow(clippy::await_holding_lock)]
use clap::Parser;
use lazy_static::lazy_static;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use reqwest::{Client, Url};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::RwLock
};
use futures::future;
lazy_static! {
    static ref WEBPAGES_TO_RIP: RwLock<HashSet<String>> = RwLock::new(HashSet::new());
    static ref ALREADY_RIPPED: RwLock<HashSet<String>> = RwLock::new(HashSet::new());
    static ref RESOURCES_TO_RIP: RwLock<HashSet<String>> = RwLock::new(HashSet::new());
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct RipArgs {
    ///The root for the ripping function, ex. https://en.wikipedia.org
    #[clap(short, long, value_parser)] 
    root: String,
    
    ///The suffix of the domain pointing to the landing page of the mediawiki instance, ex. /wiki/Main_Page (Taken from https://en.wikipedia.org/wiki/Main_Page)
    #[clap(short, long, value_parser)] 
    starting_page: String
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = RipArgs::parse();
    if Path::new("ripped").exists() {
        fs::remove_dir_all("ripped").unwrap();
    }
    fs::create_dir("ripped").unwrap();
    {
        let mut rip_writer = WEBPAGES_TO_RIP.write().unwrap();
        rip_writer.insert(args.starting_page.to_owned());
    }
    let client = Client::new();
    let root = Url::parse(args.root.as_str()).unwrap();
    loop {
        let copy = WEBPAGES_TO_RIP.read().unwrap().clone();
        let iterator = copy.iter();

        if iterator.len() == 0 {
            break;
        }

        future::join_all(iterator.par_bridge().map(|link|{
            println!("RIPPING PAGE: {}", &link);
            rip(&root, link, &client)
        }).collect::<Vec<_>>()).await;
    }
    let res = RESOURCES_TO_RIP.read().unwrap().clone();

    future::join_all(res.iter().par_bridge().map(|link|{
        println!("RIPPING RESOURCE: {}", &link);
        rip_res(&root, link, &client)
    }).collect::<Vec<_>>()).await;
   
    Ok(())
}

async fn rip(
    root: &Url,
    url_suffix: &str,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    {
        let mut writer = ALREADY_RIPPED.write().unwrap();
        writer.insert(url_suffix.to_owned());

        let mut ripper_writer = WEBPAGES_TO_RIP.write().unwrap();
        ripper_writer.remove(url_suffix);
    }
        
    let path = str_to_path_buf(url_suffix).unwrap();
    let url = root.join(url_suffix).unwrap();
    if !path.exists(){
       fs::create_dir_all(path.parent().unwrap()).unwrap(); 
    }
    let mut file = File::create(path).unwrap();
    //let mut file = File::create(format!("ripped/{}", sanitized_url)).unwrap();
    let text = client.get(url).send().await?.text().await?;
    file.write_all(text.as_bytes())?;

    //scan for all valid href= links

    let mut links = text
        .match_indices("href=")
        .map(|tuple| {
            let mut position = tuple.0 + 6;
            let mut current_char: char = text.chars().nth(position).unwrap();
            let mut acc = String::new();

            loop {
                acc.push(current_char);
                position += 1;
                current_char = text.chars().nth(position).unwrap();
                if current_char == '"' {
                    break;
                }
            }
            acc
        })
        .collect::<Vec<String>>();

    links.retain(|unknown| {
        unknown.starts_with('/')
            && !(unknown.contains(':') || unknown.contains('?') || unknown.contains('#'))
    });
    //scan for all valid resources

    let mut resources = text
        .match_indices("src=")
        .map(|tuple| {
            let mut position = tuple.0 + 5;
            let mut current_char: char = text.chars().nth(position).unwrap();
            let mut acc = String::new();

            loop {
                acc.push(current_char);
                position += 1;
                current_char = text.chars().nth(position).unwrap();
                if current_char == '"' {
                    break;
                }
            }
            acc
        })
        .collect::<Vec<String>>();

    resources.retain(|unknown| {
        unknown.starts_with('/')
            && (unknown.ends_with(".jpg")
                || unknown.ends_with(".png")
                || unknown.ends_with(".jpeg")
                || unknown.ends_with(".gif")
                || unknown.ends_with(".svg"))
    });
    //they're in diffrent scopes so that we drop them as soon as possible.
    {
        let mut res_writer = RESOURCES_TO_RIP.write().unwrap();
        res_writer.extend(resources.into_iter());
    }
    {
        let already_ripped = ALREADY_RIPPED.read().unwrap();
        links.retain(|link| !already_ripped.contains(link));
    }
    {
        let mut rip_writer = WEBPAGES_TO_RIP.write().unwrap();
        rip_writer.extend(links.into_iter());
    }

    Ok(())
}

async fn rip_res(
    root: &Url,
    url_suffix: &str,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error>> {
    //sanitized_url.retain(|c| c.is_alphanumeric() || c == '.');
    let path = str_to_path_buf(url_suffix).unwrap();
    let url = root.join(url_suffix).unwrap();
    if !path.exists(){
       fs::create_dir_all(path.parent().unwrap()).unwrap(); 
    }
    let mut file = File::create(path).unwrap();
    //let mut file = File::create(format!("ripped/{}", sanitized_url)).unwrap();
    let bytes = client.get(url).send().await?.bytes().await?;
    file.write_all(bytes.to_vec().as_slice())?;
    Ok(())
}


fn str_to_path_buf(string: &str) -> Result<PathBuf, Box<dyn std::error::Error>>{
    if string.starts_with('/') {
        Ok(PathBuf::from(format!("ripped{}",string)))
    }else {
        Ok(PathBuf::from(format!("ripped/{}",string)))
    }   
}
