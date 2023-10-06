use std::{collections::HashMap, env::VarError};

pub fn remove_first_last(s: &str) -> &str {
    let mut chars = s.chars();
    chars.next();
    chars.next_back();
    chars.as_str()
}

/**
 * Gets a variable from the environment or from a `HashMap`.
 *
 * # Arguments
 *
 * * `key`: Environment variable key
 * * `defines`: `HashMap` of local defines.
 *
 * # Errors
 *
 * This function will return an error if the environment variable is not
 * define and it is not found in the provided `HashMap`.
 *  
 */
pub fn get_environment_variable(
    key: &str,
    defines: &HashMap<&str, &str>,
) -> Result<String, VarError> {
    if let Ok(val) = std::env::var(key) {
        Ok(val)
    } else if let Some(&val) = defines.get(key) {
        Ok(val.to_owned())
    } else {
        Err(VarError::NotPresent)
    }
}
