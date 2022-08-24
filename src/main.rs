#![feature(atomic_from_mut, inline_const)]
use error_chain::error_chain;
use rand::prelude::*;
use regex::Regex;
use std::thread;
use std::sync::atomic::*;
use clap::Parser;

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

/// Scrape the Sims 4 Mod website for all the mods
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Number of threads to run
    #[clap(short, long, value_parser, default_value_t = 10)]
    threads: u8,
    /// Number of mods to scrape
    #[clap(short, long, value_parser, default_value_t = 10)]
    mods: u8,
    /// Remove the sims4mods database, remove the entire database
    #[clap(short, long, parse(from_occurrences))]
    remove: i32,
    /// Verbose output
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
    /// check the database for duplicants
    #[clap(short, long, value_parser,  default_value_t = false)]
    check: bool,

    /// save the id of the mods
    #[clap(short, long, value_parser,  default_value_t = false)]
    save: bool,

    ///import the mods into the database from "ids"
    #[clap(short, long, value_parser,  default_value_t = false)]
    import: bool,
    
}

static GLOBAL_S4MODS_MAX: i32 = 100;
static GLOBAL_THREAD_MAX: i32 = 10;
static GLOBAL_TIMES_AMOUNT: AtomicI32 = AtomicI32::new(0);
static GLOBAL_FAIL_AMOUNT: AtomicI32 = AtomicI32::new(0);
static GLOBAL_SUCCESSES_AMOUNT: AtomicI32 = AtomicI32::new(0);
static GLOBAL_S4MODS_AMOUNT: AtomicI32 = AtomicI32::new(0);


#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mut handlers = Vec::new();
    for i in 0..=GLOBAL_THREAD_MAX {
        let handler = threadbuild(i);
        handlers.push(handler)
    }
    for h in handlers {
        h.join().unwrap();
    }
    
    Ok(())
    
}

fn threadbuild(i: i32) -> std::thread::JoinHandle<std::string::String>{
    let builder = thread::Builder::new().name(format!("Thread: {}", i));

    let handler = builder.spawn(move || {
        println!("thread started = {}", thread::current().name().unwrap());
        
        while GLOBAL_TIMES_AMOUNT.load(Ordering::SeqCst) < GLOBAL_S4MODS_MAX {
            let mut rng = thread_rng();
            let x: i32 = rng.gen_range(0..2000000);
            let url= format!("https://www.thesimsresource.com/downloads/{}/", x);
            
            //println!("url:\n{:#?}", url);
            let res = reqwest::blocking::get(url).unwrap();
            //println!("Checking: {}", x);
            //println!("{}", res.status());
            if res.status() == 404 {
                //x = rng.gen_range(0..2000000);
                GLOBAL_FAIL_AMOUNT.fetch_add(1, Ordering::SeqCst);
            } else {
                statussucess(res, i)
            }
        }
        thread::current().name().unwrap().to_owned()
    }).unwrap();

    return handler;
}

fn statussucess(res: reqwest::blocking::Response, i: i32){
    let url = res.url();
    GLOBAL_SUCCESSES_AMOUNT.fetch_add(1, Ordering::SeqCst);
    let con = sqlite::open("./sims4.sqlite3").unwrap();
    con.execute("CREATE TABLE IF NOT EXISTS sims4mods (id INTEGER, name TEXT, category TEXT, author TEXT, url TEXT, game TEXT)").map_err(|err| println!("{:?}", err)).ok();
    let re = Regex::new(r"sims4").unwrap();
    
    let s4check = re.is_match(url.as_str());

    if s4check == true{
        GLOBAL_S4MODS_AMOUNT.fetch_add(1, Ordering::SeqCst);
        // increment the times variable by 1
        GLOBAL_TIMES_AMOUNT.fetch_add(1, Ordering::SeqCst);
        
        // print the times variable for debugging
        println!("Amount: {}, thread: {}", GLOBAL_TIMES_AMOUNT.load(Ordering::SeqCst), i);


        urlregex(&url.as_str());
    }

}

fn urlregex(url: &str) {
    let con = sqlite::open("./sims4.sqlite3").unwrap();
    con.execute("CREATE TABLE IF NOT EXISTS sims4mods (id INTEGER, name TEXT, category TEXT, author TEXT, url TEXT, game TEXT)").map_err(|err| println!("{:?}", err)).ok();
    let idregex = Regex::new(r"id/").unwrap();
    let pull: Vec<_> = idregex.split(url).into_iter().collect();
    //println!("{:?}", pull);
    let id = pull[1].strip_suffix(r"/").unwrap();
    let categoryregex = Regex::new(r"category/").unwrap();
    let pull: Vec<_> = categoryregex.split(url).into_iter().collect();
    let categoryregex = Regex::new(r"/title").unwrap();
    let pull: Vec<_> = categoryregex.split(pull[1]).into_iter().collect();

    let category = pull[0];
    let category = category.replace("-", " ");
    // remove the surrounding characters from the category of the mod and replace the hyphens with spaces

    // regex search the url for the name of the mod
    let titleregex = Regex::new(r"/title.").unwrap();
    let pull: Vec<_> = titleregex.split(url).into_iter().collect();
    let titleregex = Regex::new(r"/id").unwrap();
    let pull: Vec<_> = titleregex.split(pull[1]).into_iter().collect();
    // remove the surrounding characters from the name of the mod and replace the hyphens with spaces
    let name = pull[0].replace("-", " ");

    let authorregex = Regex::new(r"members/|artists/.").unwrap();
    let pull: Vec<_> = authorregex.split(url).into_iter().collect();
    let authorregex = Regex::new(r"/downloads").unwrap();
    let pull: Vec<_> = authorregex.split(pull[1]).into_iter().collect();

    let author = pull[0];

    con.execute(format!("INSERT INTO sims4mods VALUES ('{}', '{}', '{}', '{}', '{}', '{}')", id, name, category, author, url, "Sims 4")).map_err(|err| println!("{:?}", err)).ok();
    
}   
