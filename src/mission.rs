// mission.rs
//

use super::errors;
use crate::errors::INSUFFICIENT_SPACE_ERROR;
use crate::{time_local, time_warped};
use core::convert::TryInto;
use serde::de;
use std::any::Any;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::str::FromStr;

pub enum Value {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

pub struct Pair {
    pub name: String,
    pub value: Value,
}
pub enum Token {
    QuotedString(String),
    Comment(String),
    Value(Value),
    Variable(String),

    Define(Pair),
    BeginProcessConfig(String),
    EndProcessConfig(),
    String(String),
}

pub struct Mission {
    pub(crate) file_name: String,
    pub(crate) values: Value,
}

impl Mission {
    pub fn test(&self) {}

    /// @TODO: Still needs to be populated
    pub fn get(&self, key: &str) -> Option<&Value> {
        let d = Token::Define(Pair {
            name: String::from("test"),
            value: Value::Integer(12),
        });
        if let Token::Define(pair) = d {}
        None
    }
}

// @TODO: Need to provide a way to interate over a mission file.
pub struct AsyncMissionReader {}

impl AsyncMissionReader {
    // @TODO:
    pub fn get_next_valid_line(&self, reader: &mut BufReader<File>) -> &str {
        let mut line = String::new();
        if let Ok(bytes_read) = reader.read_line(&mut line) {}
        ""
    }
    // @TODO:
    pub fn seek(line: u32) -> bool {
        false
    }

    /// Checks if the specificed line is a comment
    /// Returns: true if the line starts with a comments
    pub fn is_comment(line: &str) -> bool {
        line.trim_start().starts_with("//")
    }
}

impl std::ops::Index<&str> for Mission {
    type Output = Value;

    fn index(&self, index: &str) -> &Value {
        // @TODO: We probably need to create our own value class
        // to allow value from throwing an exception if the index
        // isn't found
        self.values.get(index).expect("index not found")
    }
}

impl std::ops::IndexMut<&str> for Mission {
    fn index_mut(&mut self, index: &str) -> &mut Value {
        self.values.get_mut(index).expect("index not found")
    }
}

impl std::ops::Index<String> for Mission {
    type Output = Value;

    fn index(&self, index: String) -> &Value {
        self.values.get(&index).expect("index not found")
    }
}

impl std::ops::IndexMut<String> for Mission {
    fn index_mut(&mut self, index: String) -> &mut Value {
        self.values.get_mut(&index).expect("index not found")
    }
}

// ---------------------------------------------------------------------------
//  Tests

#[cfg(test)]
mod tests {

    use std::fs::read_dir;
    use std::fs::File;
    use std::io::BufRead;
    use std::io::BufReader;
    use std::path::{Path, PathBuf};

    use crate::errors::*;

    use crate::mission::Mission;
    use toml::Value;

    #[test]
    #[ignore]
    fn test_data_type() {
        let test_mission = r###"
// Test Mission File
ServerHost   = localhost
ServerPort   = 9000
Community    = alpha
MOOSTimeWarp = 1

// MIT Sailing Pavilion
LatOrigin  = 42.35846207515723
LongOrigin = -71.08774014042629

//------------------------------------------
// Antler configuration  block
ProcessConfig = ANTLER
{
  MSBetweenLaunches = 200 
  ExecutablePath = system // System path
  Run = MOOSDB          @ NewConsole = 
  Run = pLogger         @ NewConsole = true
  Run = uSimMarine	    @ NewConsole = false
  Run = pMarinePID      @ NewConsole = false
  Run = pHelmIvP        @ NewConsole = false
  Run = pMarineViewer	@ NewConsole = false
  Run = uProcessWatch	@ NewConsole = false
  Run = pNodeReporter	@ NewConsole = false
  Run = uMemWatch       @ NewConsole = false
}

//------------------------------------------
// uMemWatch config block

ProcessConfig = uMemWatch                                       
{                                                               
  AppTick   = $(POP) // Test
  CommsTick = 4                                                 
                                                                
  absolute_time_gap = 1   // In Seconds, Default is 4

  watch_only = pHelmIvP,pMarineViewer
  test_value = This  is   a test // Test Comment
}

"###;

        let mut mission_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        mission_dir.push("resources/test/mission");

        if mission_dir.is_dir() {
            for entry in read_dir(mission_dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                println!("{}", path.to_str().unwrap());
            }
        }

        let mut mission = Mission {
            file_name: String::from(""),
            values: toml::value::Table::new(),
        };

        mission
            .values
            .insert(String::from("ServerHost"), Value::from("test"));

        // assert_eq!(mission["ServerHost"].as_str(), Some("test"));
        // assert_eq!(mission[String::from("ServerHost")].as_str(), Some("test"));

        // let t = &mission["test"];

        mission.test();

        let value = "foo = 'bar'".parse::<Value>().unwrap();

        assert_eq!(value["foo"].as_str(), Some("bar"));

        let value = "[foo]\n  bar = 12".parse::<Value>().unwrap();
        assert_eq!(value["foo"]["bar"].as_integer(), Some(12));

        let f = std::fs::File::open("/tmp/config.yaml").unwrap();
        let reader = BufReader::new(f);
        let value: serde_yaml::Value = serde_yaml::from_reader(reader).unwrap_or_default();
        assert_eq!(value["foo"]["bar"].as_i64(), Some(12));

        // This works and returns an empty value
        let t = &value["test"]["abc"];

        let value = std::fs::read_to_string("/tmp/config.toml")
            .unwrap_or_default()
            .parse::<Value>()
            .unwrap();

        // This does not work and throws an exception
        //let t = &value["test"];
        let t = &value["foo"];

        assert_eq!(value["foo"]["bar"].as_integer(), Some(12));
    }

    #[test]
    fn test_json() {
        use serde_json::Value;
        let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ],
            "MOOSTimeWarp": 3.0,
            "MyApp":
            {
                "AppTick": 4.0
            }
        }"#;

        // Parse the string of data into serde_json::Value.
        let v: Value = serde_json::from_str(data).unwrap();

        let time_warp = v["MOOSTimeWarp"].as_f64().unwrap_or(1.0);
        let app_tick = v["MyApp"]["AppTick"].as_f64().unwrap_or(1.0);

        println!("JSON: {} ", serde_json::to_string_pretty(&v).unwrap());
        println!("TimeWarp: {}", time_warp);
        println!("AppTick: {}", app_tick);
    }

    // TODO: This test should be added back in once the parser is completed
    #[test]
    #[ignore]
    fn test_types_file() {
        let mut mission_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        mission_dir.push("resources/test/mission");
        let file = mission_dir.join(Path::new("test_types.moos"));
        println!("File: {}", file.to_str().unwrap());

        assert!(file.exists());

        let f = File::open(file).expect("test_types.moos file cannot be opened");
        let mut reader = BufReader::new(f);

        // TODO: This
        let mut iter = reader.lines();
        while let Some(Ok(line)) = iter.next() {
            println!("{}", line);
        }
    }

    // @TODO: Enable when the parse is created
    #[test]
    #[ignore]
    fn test_pass_fail_files() {
        let mut mission_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        mission_dir.push("resources/test/mission");

        if mission_dir.is_dir() {
            for entry in read_dir(mission_dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();

                let file_name = path
                    .file_name()
                    .expect("File name not found")
                    .to_str()
                    .unwrap();

                if !file_name.starts_with("pass") && !file_name.starts_with("fail") {
                    continue;
                }

                let expected_value = file_name.starts_with("pass");
                println!(
                    "Parsing: \"{}\": should {}",
                    path.to_str().unwrap(),
                    if expected_value { "pass" } else { "fail" }
                );
                // @TODO: Uncomment when the parse method has been implemented
                //assert_eq!(expected_value, parse(path));
            }
        }
    }

    #[tokio::test]
    pub async fn test_async() {
        use tokio::io::AsyncBufRead;
        use tokio::io::AsyncBufReadExt;
        use tokio::io::BufReader;

        let mut mission_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        mission_dir.push("resources/test/mission");
        let file_path = mission_dir.join(Path::new("test_types.moos"));

        let f = tokio::fs::File::open(file_path).await.unwrap();
        let mut reader = BufReader::new(f);

        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await.unwrap_or(None) {
            println!("{}", line);
        }
    }
}
