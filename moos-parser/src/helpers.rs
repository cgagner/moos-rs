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

#[macro_export]
macro_rules! vec_wrapper {
    ( $name:ident , $type:ident ) => {
        #[derive(Debug, Default)]
        pub struct $name<'lt>(Vec<$type<'lt>>);

        impl<'lt> IntoIterator for $name<'lt> {
            type Item = $type<'lt>;
            type IntoIter = std::vec::IntoIter<Self::Item>;

            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }

        impl<'lt> IntoIterator for &'lt $name<'lt> {
            type Item = &'lt $type<'lt>;
            type IntoIter = core::slice::Iter<'lt, $type<'lt>>;

            fn into_iter(self) -> Self::IntoIter {
                (&self.0).into_iter()
            }
        }

        impl<'lt> $name<'lt> {
            pub fn new() -> Self {
                Self(Vec::new())
            }

            #[inline]
            pub fn clear(&mut self) {
                self.0.clear();
            }

            #[inline]
            pub fn len(&self) -> usize {
                self.0.len()
            }

            #[inline]
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }

            #[inline]
            pub fn iter(&self) -> core::slice::Iter<'lt, $type> {
                self.0.iter()
            }

            /// Combine all of the values into one string. If there are environment
            /// variables, those will be evaluated and replaced with their values.
            #[inline]
            pub fn eval(&self) -> String {
                let rtn = "".to_owned();
                self.0
                    .iter()
                    .fold(rtn, |acc, v| acc + v.to_string().as_str())
            }

            #[inline]
            pub fn first(&self) -> Option<&$type<'lt>> {
                self.0.first()
            }

            #[inline]
            pub fn push(&mut self, value: $type<'lt>) {
                self.0.push(value);
            }

            #[inline]
            pub fn pop(&mut self) -> Option<$type<'lt>> {
                self.0.pop()
            }

            #[inline]
            pub fn last(&self) -> Option<&$type<'lt>> {
                self.0.last()
            }
        }

        impl<'lt> From<Vec<$type<'lt>>> for $name<'lt> {
            fn from(values: Vec<$type<'lt>>) -> Self {
                Self(values)
            }
        }

        impl<'lt> From<$type<'lt>> for $name<'lt> {
            fn from(value: $type<'lt>) -> Self {
                let values: Vec<$type<'lt>> = vec![value];
                Self(values.into())
            }
        }

        impl<'lt> ToString for $name<'lt> {
            fn to_string(&self) -> String {
                let rtn = "".to_owned();
                self.0
                    .iter()
                    .fold(rtn, |acc, v| acc + v.to_string().as_str())
            }
        }

        // TODO: This should be implemented here once all structs are tree nodes
        // impl<'input> crate::TreeNode for $name<'input> {
        //     fn get_start_index(&self) -> u32 {
        //         if let Some(v) = self.0.first() {
        //             v.get_start_index()
        //         } else {
        //             0
        //         }
        //     }

        //     fn get_end_index(&self) -> u32 {
        //         if let Some(v) = self.0.last() {
        //             v.get_end_index()
        //         } else {
        //             0
        //         }
        //     }
        // }
    };
}
