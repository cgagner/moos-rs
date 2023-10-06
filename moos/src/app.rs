use crate::message::{Message, MessageList};

// Do Work:
/**
 * Do Work:
 *
 * 1. Sleep if needed
 * 2. Get the time
 * 3. Check if we should use comms
 *    a. Check mail
 *    b. Sort the mail by some sorter
 *    c. OnNewMail (private)
 *    d. OnNewMail
 *    e. Increment mail count
 *    f. Implicitly, somewhere in here need to check the connection
 *    h. If its connected or if iterate without comms is set
 *       1. Iterate (private)
 *       2. Check if we need to iterate
 *          a. OnIteratePrepare()
 *          b. Iterate()
 *          c. OnIterateComplete()
 *          d. If any of these fail, check to see if need to
 *             quit if iterate fails.
 *       3. Increment iterate counter
 * 4. Do the iterate loop
 */

pub struct AppError {}

/**
 * Iterate mode used by the MOOS application.
 */
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IterateMode {
    /// Process new mail and iterate at a regular interval determined by
    /// `AppTick`. The `iterate` will always be called after `on_new_mail`.
    #[default]
    RegularIterateAndMail,
    /// Process new mail and iterate based on new messages arriving. The
    /// `iterate` method will always be called after `on_new_mail`. However,
    /// `iterate` will be called at least `AppTick` per second even if mail
    /// does not arrive.
    CommsDrivenIterateAndMail,
    /// Iterate is called regularly based on `AppTick` and new mail is
    /// processed as it arrives. The `iterate` method is decoupled from the
    /// `on_new_mail` method.
    RegularIterateAndCommsDrivenMail,
}

pub trait App {
    /// Called when the application receives new mail
    fn on_new_mail(&mut self, new_mail: &MessageList) -> Result<(), AppError>;

    /// Called prior to `iterate` to allow the app to perform any needed
    /// preparations. Most often this method just returns `Ok(())`
    ///
    /// # Arguments
    ///
    /// * `time`: Current MOOS time of the iteration in seconds. This will be
    ///    warped if there is a time warp.
    fn on_iterate_prepare(&mut self, time: f64) -> Result<(), AppError> {
        Ok(())
    }

    /// Called periodically to perform the process loop of the application
    ///
    /// # Arguments
    ///
    /// * `time`: Current MOOS time of the iteration in seconds. This will be
    ///    warped if there is a time warp.
    fn iterate(&mut self, time: f64) -> Result<(), AppError>;

    /// Called after to `iterate` to allow the app to perform any needed
    /// cleanup. Most often this method just returns `Ok(())`
    ///
    /// # Arguments
    ///
    /// * `time`: Current MOOS time of the iteration in seconds. This will be
    ///    warped if there is a time warp.
    fn on_iterate_complete(&mut self, time: f64) -> Result<(), AppError> {
        Ok(())
    }

    /// Called when the application connects to the MOOSDB.
    fn on_connect_to_server(&mut self) -> Result<(), AppError> {
        Ok(())
    }

    /// Called when the application disconnects from the MOOSDB.
    fn on_disconnect_from_server(&mut self) -> Result<(), AppError> {
        Ok(())
    }

    /// Called during application startup after to connecting to the MOOSDB.
    /// This method is called once at the start of the program and should be
    /// used preform any one-time initializations such as parse a mission
    /// file.
    fn on_start_up(&mut self) -> Result<(), AppError>;

    /// Returns the name of the application.
    fn get_app_name(&self) -> String;

    /// Set the iterate mode of the application.
    ///
    /// # Arguments
    ///
    /// * `mode`: Iterate mode of the application.
    fn set_iterate_mode(&mut self, mode: IterateMode) -> Result<(), AppError>;

    /// Get the iterate mode of the application.
    ///
    /// See: `IterateMode`
    fn get_iterate_mode(&self) -> IterateMode;

    // Called to generate a report on the current state of the application.
    // This report is used for AppCasting.
    //fn build_report(&mut self) -> Result<(), AppError>;
}
