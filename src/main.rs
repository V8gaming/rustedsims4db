#![feature(atomic_from_mut, inline_const)]
#![feature(path_try_exists)]
use error_chain::error_chain;
use rand::prelude::*;
use regex::Regex;
use std::path::Path;
use std::thread;
use std::sync::atomic::*;
use clap::Parser;
use std::fs;
use std::env::current_dir;

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
    threads: i32,
    /// Number of mods to scrape
    #[clap(short, long, value_parser, default_value_t = 10)]
    mods: i32,
    /// Remove the sims4mods database, remove the entire database
    #[clap(short, long, parse(from_occurrences))]
    remove: i32,
    /// Verbose output
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
    /// check the database for duplicants
    #[clap(short, long, parse(from_occurrences))]
    check: i32,

    /// save the id of the mods
    #[clap(short, long, parse(from_occurrences))]
    save: i32,

    ///import the mods into the database from "ids"
    #[clap(short, long, parse(from_occurrences))]
    import: i32,
    
}

// initalise the global varibles as: Amount of sims4 mods wanted, Threads wanted, Amount of Sims4 mods found, Amount of 404, amount of non 404, Amount of Sims4 mods found.
static GLOBAL_TIMES_AMOUNT: AtomicI32 = AtomicI32::new(0);
static GLOBAL_FAIL_AMOUNT: AtomicI32 = AtomicI32::new(0);
static GLOBAL_SUCCESSES_AMOUNT: AtomicI32 = AtomicI32::new(0);
static GLOBAL_S4MODS_AMOUNT: AtomicI32 = AtomicI32::new(0);


#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let database = format!("{}\\sims4.sqlite3", current_dir().unwrap().display());
    if args.remove > 0 {
        remove(args.remove, database.clone());
    }
    if args.import == 1 {
        if Path::new(&database).try_exists().unwrap() {
            importids();
        }
    }
    let mut handlers = Vec::new();
    for i in 0..=args.threads {
        let handler = threadbuild(i, args.mods, args.save);
        handlers.push(handler)
    }
    for h in handlers {
        h.join().unwrap();
    }
    if args.check == 1 {
        checkids(args.save, database);
    }

    Ok(())
    
    
}

fn threadbuild(i: i32, mods: i32, save: i32) -> std::thread::JoinHandle<std::string::String>{
    let builder = thread::Builder::new().name(format!("Thread: {}", i));

    let handler = builder.spawn(move || {
        println!("thread started = {}", thread::current().name().unwrap());
        
        while GLOBAL_TIMES_AMOUNT.load(Ordering::SeqCst) < mods {
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
                statussucess(res, i, save)
            }
        }
        thread::current().name().unwrap().to_owned()
    }).unwrap();

    return handler;
}

fn statussucess(res: reqwest::blocking::Response, i: i32, save: i32){
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


        urlregex(&url.as_str(), save);
    }

}

fn urlregex(url: &str, save: i32) {
    let con = sqlite::open("./sims4.sqlite3").unwrap();
    // create a table
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
    // format and execute the data into the database
    con.execute(format!("INSERT INTO sims4mods VALUES ('{}', '{}', '{}', '{}', '{}', '{}')", id, name, category, author, url, "Sims 4")).map_err(|err| println!("{:?}", err)).ok();
    if save == 1 {
        con.execute("CREATE TABLE IF NOT EXISTS ids (id INTEGER)").map_err(|err| println!("{:?}", err)).ok();
        con.execute(format!("INSERT INTO ids (id) VALUES ({})", id)).map_err(|err| println!("{:?}", err)).ok();

    };
}

fn remove(strength: i32, database: String) {
    if strength == 1 {
        let con = sqlite::open(database).unwrap();
        con.execute("DROP TABLE IF EXISTS sims4mods").map_err(|err| println!("{:?}", err)).ok();
    } else if strength == 2 {
        let exists = Path::new(&database).try_exists().unwrap();
        if exists {
            //println!("{}", exists);
            fs::remove_file(database).unwrap()
        }
    }
}

fn importids() {
    todo!()
}

fn checkids(save: i32, database: String) {
    // delete duplicates in ids
    if save == 1 {
        let con = sqlite::open(database.clone()).unwrap();
        con.iterate("SELECT * FROM ids GROUP BY id HAVING COUNT(*) > 1", |pairs| {
            for &value in pairs.iter() {
                println!("{:?}", value);
                con.execute(format!("DELETE FROM ids WHERE id = {}", value.0)).map_err(|err| println!("{:?}", err)).ok();
                con.execute(format!("INSERT FROM ids WHERE id = {}", value.0)).map_err(|err| println!("{:?}", err)).ok();
            }
            true
        }).unwrap();
    }
    let mut _duplicates: Vec<&str> = Vec::new();
    let con = sqlite::open(database).unwrap();
    con.iterate("SELECT * FROM sims4mods GROUP BY id HAVING COUNT(*) > 1", |pairs| {
        for &value in pairs.iter() {
            println!("{:?}", value);
        }
        true
    }).unwrap();
}
