use crate::args::{Cli, Color, Mode, ShowOptions};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    error, mem,
    sync::mpsc::Receiver,
    time::Instant,
};

use moos::async_client::{AsyncClient, Publish};
use moos::{app::IterateMode, message::Message};
use moos::{
    app::{App as AppTrait, AppError},
    message::MessageList,
};

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

struct Config {
    use_comms: bool,
    // "CatchCOmmandMessages"
    iterate_without_comms: bool,
    sort_mail_by_time: bool,
    filter_commands: bool,
    quit_on_iterate_fail: bool,
    log_level: log::Level,
    app_tick: f64,
    max_app_tick: f64,
    // Global or application - Application should take priority
    time_warp: f64,
    time_warp_delay_factor: f64,
    terminal_report_interval: f64,
    max_appcast_evetns: f64,
    max_appcast_run_warnings: f64,

    // Global
    // "ServerHost" or
    server_host: String,
    // "ServerPort"
    server_port: u16,
    // "Community"
    community: String,
    // "TERM_REPORTING"
    term_reporting: bool,
}

pub struct MoosApp {
    running: bool,
    client: Option<AsyncClient>,
    iterate_mode: IterateMode,
    inbox: Option<Receiver<Message>>,
    iterate_count: usize,
    mail_count: usize,
    startup_time: f64,
    use_moos_comms: bool,
}

impl MoosApp {
    fn new() -> Self {
        Self {
            running: false,
            client: None,
            iterate_mode: IterateMode::default(),
            inbox: None,
            iterate_count: 0,
            mail_count: 0,
            startup_time: 0.0,
            use_moos_comms: true,
        }
    }
}

impl AppTrait for MoosApp {
    fn on_new_mail(&mut self, new_mail: &MessageList) -> Result<(), moos::app::AppError> {
        todo!()
    }

    fn iterate(&mut self, time: f64) -> Result<(), moos::app::AppError> {
        todo!()
    }

    fn on_start_up(&mut self) -> Result<(), moos::app::AppError> {
        todo!()
    }

    fn get_app_name(&self) -> String {
        todo!()
    }

    fn set_iterate_mode(
        &mut self,
        mode: moos::app::IterateMode,
    ) -> Result<(), moos::app::AppError> {
        if !self.running {
            self.iterate_mode = mode;
            Ok(())
        } else {
            Err(AppError {})
        }
    }

    fn get_iterate_mode(&self) -> moos::app::IterateMode {
        self.iterate_mode
    }
}

impl MoosApp {
    // -----------------------------------------------------------------------
    // Methods that need to get moved into the derive macro
    // -----------------------------------------------------------------------

    fn configure(&mut self) -> Result<(), AppError> {
        // Get the host and port from either the cli or from the config file

        // Command Message: called when a command message (<MOOSNAME>_CMD) is recieved by the application

        /* Command-line or Configuration file
            moos_filter_command - CatchCommandMessages <- Used by pSHare and pLogger,
            moos_iterate_no_comms
            moos_no_comms
            moos_no_sort_mail - or SortMailByTime

            moos_quit_on_iterate_fail


            // This should just be the log level...
            moos_quiet

            // Not going to support suicide...
            moos_suicide_disable
            moos_suicide_print
        */

        /* Command-line or Application Param
           moos_app_tick -- or APPTICK
           moos_iterate_mode
           moos_max_app_tick

           moos_time_warp - MOOSTimeWarp
           moos_tw_delay_factor
            // Not going to support suicide...
           moos_suicide_channel
           moos_suicide_phrase
           moos_suicide_port
        */

        /* Standard Application Configuration options
           APPTICK
           COMMSTICK // Deprecated
           CatchCommandMessages
           ITERATEMODE
           MAXAPPTICK
           TERM_REPORT_INTERVAL <- AppCasting
           MAX_APPCAST_EVENTS <- AppCasting
           MAX_APPCAST_RUN_WARNINGS <- AppCasting

        */

        /* Standard Global Configuration
           MOOSTimeWarp
           SERVERHOST
           SERVERPORT
           UseMOOSComms // Do MOOS apps really not use MOOS COMMS?
           COMMUNITY - From AppCasting
           TERM_REPORTING - From AppCasting
        */

        // Read all of the standard options from the mission file

        Ok(())
    }

    fn configure_comms(&mut self) -> Result<(), AppError> {
        Ok(())
    }

    fn run(&mut self, name: &str, mission_file: &str) -> Result<(), AppError> {
        // Process Command Line Arguments

        // Configure

        // Mark Start time

        self.startup_time = moos::get_time_warp();

        // If use comms, setup comms and wait for connection

        // OnStartupPrepare, OnStartup, OnStartupComplete

        // While !quit_requested && self.do_work() -> do work

        // Check if quit on iteration and do work success

        // Close comms

        while let Ok(()) = self.do_work() {}

        Ok(())
    }

    fn do_work(&mut self) -> Result<(), AppError> {
        Ok(())
    }

    fn _check_mail(&mut self) -> Result<(), AppError> {
        if let Some(inbox) = &self.inbox {
            let mut messages: MessageList = inbox.try_iter().collect();
            // Sort by time, then by name
            messages.sort_by(|a, b| {
                if a.time() < b.time() {
                    Ordering::Less
                } else if a.time() > b.time() {
                    Ordering::Greater
                } else {
                    a.key().cmp(b.key())
                }
            });

            // TODO: pass messages to on_new_mail

            self.on_new_mail(&messages)?;

            self.mail_count += 1;
        }
        Ok(())
    }
}

pub struct App {
    cli: Cli,

    /// Is the application running?
    pub running: bool,
    /// counter
    pub counter: usize,
    pub data: Vec<Vec<&'static str>>,
    pub color_map: HashMap<String, Color>,
    pub columns: HashSet<ShowOptions>,
}

impl App {
    // Create a new application and take ownership of the Cli
    pub fn new(mut cli: Cli) -> Self {
        // TODO: This is temporary
        let data = vec![
            vec!["Alice", "25"],
            vec!["Bob", "30"],
            vec!["Charlie", "35"],
            vec!["Dave", "40"],
            vec!["Eve", "45"],
            vec!["Frank", "50"],
            vec!["Grace", "55"],
            vec!["Harry", "60"],
        ];

        let mut color_map: HashMap<String, Color> = cli.get_color_map();

        let mut available_colors = vec![
            Color::Blue,
            Color::Cyan,
            Color::Green,
            Color::Magenta,
            Color::Red,
        ];

        let used_colors: HashSet<Color> =
            color_map.values().into_iter().map(|c| c.clone()).collect();

        available_colors.retain(|&c| !used_colors.contains(&c));

        let mut get_available_color = || -> Color {
            if let Some(c) = available_colors.first() {
                let c = c.clone();
                available_colors.remove(0);
                c
            } else {
                Color::Any
            }
        };

        color_map.extend(
            cli.colorany
                .iter()
                .map(|s| (s.to_owned(), get_available_color())),
        );

        let columns: HashSet<ShowOptions> = cli.show.clone().into_iter().collect();

        App {
            cli,
            running: true,
            counter: 0,
            data,
            color_map,
            columns,
        }
    }

    pub fn run(
        &self,
        mission_file: Option<String>,
        app_name: Option<String>,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn increment_counter(&mut self) {
        if let Some(res) = self.counter.checked_add(1) {
            self.counter = res;
        }
    }

    pub fn decrement_counter(&mut self) {
        if let Some(res) = self.counter.checked_sub(1) {
            self.counter = res;
        }
    }

    #[inline]
    pub fn toggle_column(&mut self, option: ShowOptions) {
        if self.columns.contains(&option) {
            self.columns.remove(&option);
        } else {
            self.columns.insert(option);
        }
    }

    #[inline]
    pub fn toggle_source_column(&mut self) {
        self.toggle_column(ShowOptions::Source)
    }

    #[inline]
    pub fn toggle_time_column(&mut self) {
        self.toggle_column(ShowOptions::Time)
    }

    #[inline]
    pub fn toggle_community_column(&mut self) {
        self.toggle_column(ShowOptions::Community)
    }

    #[inline]
    pub fn toggle_aux_column(&mut self) {
        self.toggle_column(ShowOptions::Aux)
    }
}
