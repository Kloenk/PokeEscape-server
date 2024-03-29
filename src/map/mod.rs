use colored::*;
use semver::Version;
use serde_derive::Serialize;
use std::collections::HashMap;
use std::fs;
use std::str::FromStr;
use toml::Value;

use super::error::Error;

#[doc(inline)]
pub use super::error::Result;

/// defines the widht of the map
///
/// creates warning and 0 an the right end if map in file is smaller,
/// or crops the right side if map is bigger
pub const WIDTH: usize = 28;

/// struct holding all informations of the toml file
// this is not a deserialisable struct, so every map can live at top level
pub struct MapPlaces {
    /// defines the version of the toml file
    p_version: Version,

    /// stores available (set for loading) maps
    p_maps: HashMap<String, MapInfo>,
}

impl MapPlaces {
    /// Loads toml file and deserialize it into `MapPlaces`
    ///
    /// # Arguments
    /// - `file` -> give the path of the toml file to load
    /// - `verbose` -> if set true shows status of loading
    pub fn new(file: &str, verbose: bool) -> Result<Self> {
        if verbose {
            println!("Loading {} from {}", "Maps".green(), file.blue());
        }

        let file = fs::read_to_string(file)?; //FIXME: testing
        let content: Value = toml::from_str(file.as_str())?;

        if content.get("Maps") == None {
            return Err(Error::new_field_not_exists("Maps".to_string()));
        }

        let version = match content["Maps"]["version"].as_str() {
            Some(ver) => ver,
            None => return Err(Error::new_field_not_exists("Maps.version".to_string())),
        };
        let version = Version::from_str(version)?;

        let mut maps = HashMap::new();

        if version < Version::new(99, 99, 99) {
            // check in reserve order for version
            maps = MapInfo::from_conf(&content, verbose)?;
        }
        Ok(MapPlaces {
            p_version: version,
            p_maps: maps,
        })
    }

    /// list all maps described in this `MapPlaces`
    pub fn available_maps(&self) -> Vec<String> {
        let mut vec = Vec::new();

        for (k, _) in &self.p_maps {
            vec.push(k.clone());
        }

        vec // return map names
    }

    /// returns the pointer to a specific map
    #[allow(dead_code)]
    fn get_map(&self, name: String) -> Result<Map> {
        match self.p_maps.get(&name) {
            // FIXME foo
            Some(data) => data.load_map(),
            None => Err(Error::new_field_not_exists(name)),
        }
    }

    /// return the Map with the given Name
    ///
    /// This function take a name as argument and returns the loaded Map
    /// associated with the given name
    pub fn get(&self, name: &str) -> Result<Map> {
        match self.p_maps.get(name) {
            // FIXME: version foo for random feature
            Some(data) => data.load_map(),
            None => Err(Error::new_field_not_exists(name.to_string())),
        }
    }

    /// returns the author of the map
    pub fn get_author(&self, name: &str) -> Option<String> {
        match self.p_maps.get(name) {
            Some(m) => m.author(),
            None => None,
        }
    }

    /// returns the version of the map
    pub fn version(&self) -> &Version {
        &self.p_version
    }
}

/// Map holds a map ready to send to a client
///
/// # Meanings
/// | Number | Block           | Variant                 |
/// |--------|-----------------|-------------------------|
/// | 0      | None            |                         |
/// | 1      | Solid Block     |                         |
/// | 2      | Water           |                         |
/// | 3      | Trap            |                         |
/// | 4      | Moving platform |                         |
/// | 5      | Start point     | player                  |
/// | 6      | Berry           | Energy                  |
/// | 7      | Berry           | HP                      |
/// | 8      | Berry           | XP                      |
/// | 9      | Enemy           | 100% speed              |
/// | 10     | Enemy           | 150% speed; 120% damage |
/// | 11     | Teleport        |                         |
/// | 12     | Moving platform |                         |
#[derive(Serialize)]
pub struct Map {
    p_name: String,
    p_features: Option<Vec<String>>,
    p_map: Vec<[u8; WIDTH]>,
}

impl Map {
    /// returns the name of the map
    pub fn name(&self) -> String {
        self.p_name.clone() // returns a clone
    }

    /// get size of map
    pub fn size(&self) -> String {
        format!("{}x{}", WIDTH, self.p_map.len())
    }

    /// check if the feature exists
    pub fn feature(&self, feature: &String) -> bool {
        match &self.p_features {
            Some(t) => t.contains(feature),
            None => false,
        }
    }

    /// returns a list of features of the map
    pub fn feature_list(&self) -> String {
        match &self.p_features {
            Some(t) => {
                let mut ret = String::new();
                let mut runned = false;
                for v in t {
                    if runned {
                        ret += ", ";
                    }
                    ret += v;
                    runned = true;
                }
                ret
            }
            None => "".to_string(),
        }
    }
}

/// print trait
impl std::fmt::Display for Map {
    /// standart formater for print! macro
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let json = match serde_json::to_string(self) {
            Ok(json) => json,
            Err(err) => format!("{{\"status\": 100, \"err\": \"{}\"}}", err.to_string()),
        };
        write!(f, "{}", json)
    }
}

/// this holds a single map with all of the coresponding informations
///
/// This function willnot preload the map, but loads it when used (random??)
struct MapInfo {
    /// name of the given map
    p_name: String,

    /// file to load the map from
    p_file: String,

    /// version of the maploader to use
    p_version: Version,

    /// format of the given map file
    #[allow(dead_code)]
    p_format: MapFormat,

    /// author of the map
    p_author: Option<Vec<String>>,

    /// set to true if the mapload should operate in verbose mode
    p_verbose: bool,
}

impl MapInfo {
    /// creates a new instance of the Map
    /// This function only returns the struct with the given data filled in, so there is no logic involved
    pub fn new(
        name: String,
        file: String,
        version: Version,
        format: MapFormat,
        author: Option<Vec<String>>,
        verbose: bool,
    ) -> Self {
        MapInfo {
            p_name: name,
            p_file: file,
            p_version: version,
            p_format: format,
            p_author: author,
            p_verbose: verbose,
        }
    }

    /// Read toml value and returns a HashMap with the maps specified in the toml file
    pub fn from_conf(toml: &toml::Value, verbose: bool) -> Result<HashMap<String, Self>> {
        if toml["Maps"].get("maps") == None {
            // check if maps definition exists
            return Err(Error::new_field_not_exists("Maps.maps".to_string()));
        }

        let maps_names = match toml["Maps"]["maps"].as_array() {
            // get maps definition
            Some(maps) => maps,
            None => return Err(Error::new_field_not_exists("Maps.maps".to_string())),
        };

        let mut maps = HashMap::new();

        // parse maps
        for map in maps_names {
            let map = match map.as_str() {
                Some(map) => map,
                None => {
                    eprintln!("Map name could not be converted to string");
                    continue; // parse next map
                }
            };

            if verbose {
                print!("Loading infos of map {}... ", map.green());
            }

            // check for metadata of map
            if toml.get(map) == None {
                if verbose {
                    println!("[{}]: {}", "failed".red(), "not found in config".red());
                } else {
                    eprintln!("Map {} not found in config", map.blue());
                }
                continue;
            }

            // pasrse map metadata
            let map = match Self::from_conf_one(&toml[map], map.to_string(), verbose) {
                Ok(map) => map,
                Err(err) => {
                    println!("[{}]: {}", "failed".red(), err.to_string().red());
                    continue;
                }
            };

            if verbose {
                println!(
                    "[{}]: Version: {}",
                    "Ok".green(),
                    map.version().to_string().blue()
                );
            }

            maps.insert(map.name().clone(), map); // add to hashmap
        }

        Ok(maps)
    }

    /// reads one map from the toml and and returns the Map
    ///
    /// # Parameters
    /// `toml` - The toml Value of the given Map
    /// `name` - Name of the map to read
    /// `verbose` - set to true for debug output
    fn from_conf_one(toml: &toml::Value, name: String, verbose: bool) -> Result<Self> {
        let file = match toml.get("path") {
            Some(path) => path,
            None => return Err(Error::new_field_not_exists(format!("{}.path", name))),
        };
        let file = match file.as_str() {
            Some(file) => file.to_string(),
            None => return Err(Error::new_field_not_exists(format!("{}.path", name))),
        };

        let version = match toml.get("version") {
            Some(version) => version,
            None => return Err(Error::new_field_not_exists(format!("{}.version", name))),
        };
        let version = match version.as_str() {
            Some(version) => Version::from_str(version)?,
            None => return Err(Error::new_field_not_exists(format!("{}.version", name))),
        };

        let format_str = match toml.get("format") {
            Some(format_str) => match format_str.as_str() {
                Some(format) => format,
                None => "JSON",
            },
            None => "JSON",
        };

        let author: Option<Vec<String>> = match toml.get("author") {
            // read author from toml file
            Some(a) => {
                let mut nan: bool = false;
                let mut ret = Vec::new();
                if !a.is_array() {
                    let b = match a.as_str() {
                        Some(c) => c.to_string(),
                        None => {
                            nan = true;
                            "NaN".to_string()
                        }
                    };
                    ret.push(b);
                } else if a.is_array() {
                    for v in a.as_array().unwrap() {
                        let b = match v.as_str() {
                            Some(s) => s.to_string(),
                            None => {
                                nan = true;
                                "NaN".to_string()
                            }
                        };
                        ret.push(b);
                    }
                } else {
                    nan = true
                }
                match nan {
                    true => None,
                    false => Some(ret),
                }
            }
            None => None,
        };

        let format: MapFormat;

        if format_str.to_lowercase() == "json" {
            format = MapFormat::JSON;
        } else {
            return Err(Error::new(super::error::ErrorKind::FormatNotSupported));
        }

        Ok(Self::new(name, file, version, format, author, verbose))
    }

    /// returns the name of the Map it hold information about
    pub fn name(&self) -> &String {
        &self.p_name
    }

    /// returns the version of the Map
    pub fn version(&self) -> &Version {
        &self.p_version
    }

    /// set the map into verbose mode
    #[allow(dead_code)]
    pub fn verbose(&mut self) -> &mut Self {
        self.p_verbose = true;
        self
    }

    /// returns the verbose state of the Map
    #[allow(dead_code)]
    pub fn is_verbose(&self) -> bool {
        self.p_verbose
    }

    /// returns the author of the map
    pub fn author(&self) -> Option<String> {
        let mut ret = String::new();
        let maps = match &self.p_author {
            Some(m) => m,
            None => return None,
        };
        for v in maps {
            ret += v;
        }
        Some(ret)
    }

    /// get_map returns a finish map object ready to be send
    #[allow(dead_code)]
    pub fn get_map(&self) -> Result<Map> {
        Err(Error::new_field_not_exists("code".to_string()))
    }

    /// load and return a map
    pub fn load_map(&self) -> Result<Map> {
        if self.p_verbose {
            print!(
                "Loading {} from {}...  ",
                self.p_name.green(),
                self.p_file.blue()
            );
        }

        // read json
        let file = fs::read_to_string(&self.p_file)?;
        let content: Value = serde_json::from_str(file.as_str())?;

        // get name
        let name: String = match content.get("name") {
            Some(name) => match name.as_str() {
                Some(name) => name.to_string(),
                None => return Err(Error::new_field_not_exists("name".to_string())),
            },
            None => return Err(Error::new_field_not_exists("name".to_string())),
        };
        if name != self.p_name {
            // check name
            eprintln!("Map name {} differs from name {}", name, self.p_name);
        }

        // get feature list
        let mut features: Option<Vec<String>> = None;
        let f = content.get("features");
        if f.is_some() {
            let f = f.unwrap();
            if f.is_array() {
                let mut fe = Vec::new();
                for v in f.as_array().unwrap() {
                    if !v.is_str() {
                        return Err(Error::new_field_not_exists(
                            "features is not string".to_string(),
                        ));
                    }
                    fe.push(v.as_str().unwrap().to_string());
                }
                features = Some(fe); // attach to fe
            } else if f.is_str() {
                features = Some(vec![f.as_str().unwrap().to_string()]);
            }
        }
        drop(f); // remove f
        features = match features {
            // check if it only says none
            Some(f) => {
                let mut ret: Option<Vec<String>> = Some(f.clone());
                if f.len() == 1 {
                    if f[0] == "none" {
                        ret = None;
                    }
                }
                ret
            }
            None => None,
        };

        // load map
        let mut map: Vec<[u8; WIDTH]> = Vec::new();
        let j_map = match content.get("map") {
            Some(j) => j,
            None => return Err(Error::new_field_not_exists("map".to_string())),
        };
        let j_map = match j_map.as_array() {
            Some(j) => j,
            None => return Err(Error::new_field_not_exists("map".to_string())),
        };
        let mut user_warned = false;
        for v in j_map {
            let v = match v.as_array() {
                Some(j) => j,
                None => return Err(Error::new_field_not_exists("map".to_string())),
            };
            if !user_warned {
                if v.len() < WIDTH {
                    eprintln!("\nmap smaller than {} collums", WIDTH);
                    user_warned = true;
                } else if v.len() > WIDTH {
                    eprintln!("\nmap to big");
                    user_warned = true;
                }
            }
            let mut row: [u8; WIDTH] = [0; WIDTH];
            for i in 0..WIDTH {
                let b = match v.get(i) {
                    Some(b) => b.as_integer(),
                    None => None,
                };
                let b = match b {
                    Some(b) => b as u8,
                    None => 1 as u8, // return solid block
                };
                row[i] = b;
            }
            map.push(row);
        }
        drop(j_map); // remove j_map

        let ret = Map {
            p_name: name,
            p_features: features,
            p_map: map,
        };

        if self.p_verbose {
            println!(
                "[{}]: Loaded Map {} with size {}",
                "Ok".green(),
                ret.p_name.blue(),
                ret.size().yellow()
            )
        }

        Ok(ret)
    }
}

/// map format is a enum of possible loader elements for a map file
pub enum MapFormat {
    /// JSON as the javascript object notation
    JSON,
}
