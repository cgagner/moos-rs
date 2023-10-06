use simple_logger::SimpleLogger;
use std::{collections::HashMap, str::FromStr};

pub use clap::{ArgAction, Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(value_name = "file.moos")]
    /// MOOS Mission File
    pub mission_file: Option<String>,
    #[arg(value_name = "AppName")]
    /// Application Name - Second Argument to preserve `pAntler` syntax
    pub app_name: Option<String>,
    /// Launch uXMS with the given process name rather than uXMS.
    #[arg(long, value_name = "ProcessName")]
    pub alias: Option<String>,
    /// Show ALL MOOS variables in the MOOSDB.
    #[arg(short = 'a', long)]
    pub all: bool,
    /// Ignore scope variables in file.moos.
    #[arg(short = 'c', long)]
    pub clean: bool,
    /// Display all entries where the variable, source, or community has VAR as substring.
    /// Allowable colors: blue, red, magenta, cyan, or green.
    //   Example: colormap = "IVPHELM_SUMMARY,blue,BHV_WARNING,red
    #[arg(long, value_names = &["MOOSVar,color,MOOSVar,color..."], value_parser = parse_color_map)]
    pub colormap: Vec<HashMap<String, Color>>,
    /// Display all entries where the variable, community, or source has VAR as substring.
    /// Color auto-chosen from unused colors.
    #[arg(long, value_name = "MOOSVar", value_delimiter = ',')]
    pub colorany: Vec<String>,
    /// Display example MOOS configuration block.
    #[arg(short = 'e', long)]
    pub example: bool,
    /// Allow history-scoping on variable.
    #[arg(long, value_name = "MOOSVar")]
    pub history: Option<String>,
    /// Display MOOS publications and subscriptions.
    #[arg(short = 'i', long)]
    pub interface: bool,
    /// Don't display virgin variables.
    #[arg(short = 'g', long)]
    pub novirgins: bool,
    /// Determine display mode. Paused: scope updated only on user request.
    /// Events: data updated only on change to a scoped variable.
    /// Streaming: updates continuously on each app-tick.
    #[arg(long, value_enum)]
    pub mode: Option<Mode>,
    /// Connect to MOOSDB at IP=value, not from the .moos file.
    #[arg(long, value_name = "IPAddress")]
    pub serverhost: Option<String>,
    /// Connect to MOOSDB at port=value, not from the .moos file.
    #[arg(long, value_name = "PortNumber", value_parser = clap::value_parser!(u16).range(1..) )]
    pub serverport: Option<u16>,
    /// Turn on data display in the named column, source, time, or community.
    /// All off by default enabling aux shows the auxiliary source in the source column.
    #[arg(long, value_enum, value_delimiter = ',')]
    pub show: Vec<ShowOptions>,
    /// Scope only on vars posted by the given MOOS processes.
    #[arg(long, value_name = "MOOSApp", value_delimiter = ',')]
    pub src: Vec<String>,
    /// Truncate the output in the data column.
    #[arg(long, value_name = "value")]
    // TODO: Need to check the default value from uXMS
    pub trunc: Option<u32>,
    /// Minimum real-time seconds between terminal reports.
    #[arg(long, value_name = "value", default_value = "0.6")]
    pub termint: f32,
    /// Verbosity - Use multiple times to increase the verbosity. `-v` is info. `-vv` is debug. `-vvv` is trace.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl Cli {
    /// Convert the `colormap` of `Vec<HashMap<String,Color>>` into a single
    /// `HashMap`.
    pub fn get_color_map(&self) -> HashMap<String, Color> {
        self.colormap.iter().fold(HashMap::new(), |mut acc, map| {
            acc.extend(map.clone());
            acc
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, ValueEnum)]
pub enum Mode {
    #[value(name = "paused")]
    Paused,
    #[value(name = "EVENTS")]
    Events,
    #[value(name = "streaming")]
    Streaming,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, ValueEnum)]
pub enum ShowOptions {
    //"source", "time", "community", "aux"]
    #[value(name = "source")]
    Source,
    #[value(name = "time")]
    Time,
    #[value(name = "community")]
    Community,
    #[value(name = "aux")]
    Aux,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Color {
    //  blue, red, magenta, cyan, or green - If all else fails, use any
    Blue,
    Cyan,
    Green,
    Magenta,
    Red,
    Any,
}

impl FromStr for Color {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blue" => Ok(Color::Blue),
            "cyan" => Ok(Color::Cyan),
            "green" => Ok(Color::Green),
            "magenta" => Ok(Color::Magenta),
            "red" => Ok(Color::Red),
            _ => Ok(Color::Any),
        }
    }
}

fn parse_color_map(s: &str) -> Result<HashMap<String, Color>, String> {
    let mut colors: HashMap<String, Color> = HashMap::new();
    let split_vec: Vec<&str> = s.split(',').collect();

    for i in (0..split_vec.len()).step_by(2) {
        let key = split_vec[i];
        let value = Color::from_str(split_vec.get(i + 1).copied().unwrap_or_default())
            .unwrap_or(Color::Any);
        colors.insert(key.to_owned(), value);
    }

    if !colors.is_empty() {
        return Ok(colors);
    } else {
        Err("".to_owned())
    }
}
