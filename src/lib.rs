use colored::*;
use std::net::TcpListener;
use std::process;
use structopt::StructOpt;

pub mod server;

/// threads contains the struct ThreadPool and all helper functions
pub mod threads;

/// module that holds error structures for use in `Result<T, error::Error>`
pub mod error;

/// module containing map loader
pub mod map;

/// Config is a interface designed to use with structopt on the cli, but also to run the code
///
#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub struct Config {
    /// The port to run on
    #[structopt(short = "p", long = "port", default_value = "1996")]
    pub port: u16,

    /// address to listen on
    #[structopt(short = "H", long = "host", default_value = "127.0.0.1")]
    pub host: String,

    /// Set application into verbose mode
    #[structopt(short = "v", long = "verbose")]
    pub verbose: bool,

    /// Set number of running threads
    #[structopt(short = "t", long = "threads", default_value = "8")]
    pub threads: usize,

    /// config file for maps (toml)
    #[structopt(short = "c", long = "config", default_value = "./config.toml")]
    pub config: String,
}

impl Config {
    /// Main function for the server
    pub fn run(&self) {
        println!(
            "Starting {} server on port: {}",
            "PokeEscape".green(),
            self.port.to_string().yellow()
        );
        println!(
            "{}: {}",
            "version".bold().white(),
            env!("CARGO_PKG_VERSION").blue()
        );
        if self.verbose {
            println!("Running in {} mode", "Verbose".red());
        }

        // load maps
        let maps = match map::MapPlaces::new(&self.config, self.verbose) {
            Ok(maps) => maps,
            Err(err) => {
                eprintln!("Error loading maps: {}", err.to_string().red());
                std::process::exit(20);
            }
        };

        let mut thread_pool = threads::ThreadPool::new(self.threads).unwrap_or_else(|err| {
            println!("Error creating threadPool: {}", err.red());
            process::exit(-2);
        });

        if self.verbose {
            thread_pool.verbose();
            println!(
                "created {} with {} workers",
                "ThreadPool".blue(),
                thread_pool.get_threads().to_string().green()
            );
        }

        println!(
            "listening on {}:{}",
            self.host.green(),
            self.port.to_string().green()
        );
        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port)).unwrap(); //FIXME: !!!

        for stream in listener.incoming() {
            let stream = stream.unwrap(); // FIXME: unwrap
            let conf = server::Job {
                stream,
                verbose: self.verbose,
            };

            thread_pool
                .execute(move || {
                    server::negotiate(conf).unwrap(); //FIXME: unwrap
                })
                .unwrap(); // FIXME: unwrap
        }
    }
}
