/*
 *
 * ***NOTE:*** My ADHD kicked in and I got off on a tangent building out the
 * MOOS manifests. This is a good start, but I ran into some problems that are
 * not worth solving right now. I'd like to switch back to the LSP
 * implementation since these classes cannot be used until the LSP is
 * functional.
 *
 * Problems:
 *   1. Recursive types: The `TypeInfo`` is essentially re-declaring a
 *      JSON-Object like structure for the types that we are going to handle.
 *      However, the `List` variant needs to re-use the `TypeInfo`. This
 *      had to be handled by wrapping it in a Box<TypeInfo>. I'm not sure if
 *      that is going to have some long-term side effects.
 *   2. My original design for the new manifest format was to allow creating
 *      custom types by declaring a `TypeInfo` elsewhere and re-using that
 *      in the `ObjectInfo`. This looks pretty clean in YAML. However, in the
 *      code it would mean our structure would need to change to the `TypeInfo`
 *      into a something like a `Box<dyn Into<TypeInfo>>` to store any objects
 *      that can be converted into a TypeInfo. However, that was giving me
 *      compiler errors that I don't want to deal with right now.
 *   3. Automatic serialization: I plan to create a derive macro that allows
 *      for creating an application struct that automatically handles
 *      implementing the appropriate traits to allow me to automatically
 *      generate the documentation for an application. There are some problems
 *      with this:
 *        1. Derive macros take (me) a lot of time (that I don't have) to create.
 *        2. I'd also need to create an attribute macro to associate a member
 *           inside the application struct with either MOOS mission parameters
 *           or command line arguments. More time.
 *        3. I wanted to just use `serde_yaml::to_string()`, but that will
 *           likely lead to re-serializing the `TypeInfo` for every object.
 *           I started to make a custom serialize/deserialize methods that
 *           would handle that, but without storing the state somewhere, we
 *           don't know what types have already been serialized.
 *        4. Using Includes: I planed to allow using `include` to allow for
 *           creating types in a separate file and re-used in different
 *           applications. I don't think that will work well if types are
 *           automatically serialized.
 *
 */

use chrono::{NaiveDate, Utc};
use lsp_types::SemanticTokens;
use lsp_types::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::default;

fn str_to_option(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
enum ManifestItemType {
    Application,
    CommandLineUtility,
    GraphicalUtility,
    Library,
    Behavior,
    #[default]
    Unknown,
}

// NOTE: We could use the `serde_with` crate to add a `#[skip_serializing_none]`
// attribute. The current design is to use a derive macro to automate creating
// these structures for structs that implement the corresponding trait. As
// such, pulling in an extra dependency does not make sense at this time.

#[derive(Serialize, Deserialize, Debug, Default)]
struct AppInfo {
    name: String,
    brief_description: String,
    manifest_type: ManifestItemType,
    // NaiveDate supports Serde out of the box, but uses RFC3339 format. Provide
    // some custom logic to make it use our desired format.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "my_date_format")]
    creation_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contact: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    distro: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    mission_members: Vec<ObjectInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    command_line_members: Vec<ObjectInfo>,
}

impl AppInfo {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    fn new_from_trait<T: AppInfoTrait>() -> Self {
        Self {
            name: T::NAME.to_string(),
            brief_description: T::BRIEF_DESCRIPTION.to_string(),
            manifest_type: T::MANIFEST_TYPE,
            creation_date: T::CREATION_DATE,
            description: str_to_option(T::DESCRIPTION),
            url: str_to_option(T::URL),
            license: str_to_option(T::LICENSE),
            module: str_to_option(T::MODULE),
            author: str_to_option(T::AUTHOR),
            organization: str_to_option(T::ORGANIZATION),
            contact: str_to_option(T::CONTACT),
            distro: str_to_option(T::DISTRO),

            mission_members: T::MISSION_MEMBERS,
            command_line_members: T::COMMAND_LINE_MEMBERS,
        }
    }
}

trait AppInfoTrait {
    const NAME: &'static str;
    const BRIEF_DESCRIPTION: &'static str = "";
    const MANIFEST_TYPE: ManifestItemType = ManifestItemType::Unknown;
    const CREATION_DATE: Option<NaiveDate>;
    const DESCRIPTION: &'static str = "";
    const URL: &'static str = "";
    const LICENSE: &'static str = "";
    const MODULE: &'static str = "";
    const AUTHOR: &'static str = "";
    const ORGANIZATION: &'static str = "";
    const CONTACT: &'static str = "";
    const DISTRO: &'static str = "";
    const MISSION_MEMBERS: Vec<ObjectInfo> = Vec::new();
    const COMMAND_LINE_MEMBERS: Vec<ObjectInfo> = Vec::new();
}

trait BehaviorInfoTrait {
    const NAME: &'static str;
    const BRIEF_DESCRIPTION: &'static str = "";
    const CREATION_DATE: Option<NaiveDate>;
    const DESCRIPTION: &'static str = "";
    const URL: &'static str = "";
    const LICENSE: &'static str = "";
    const MODULE: &'static str = "";
    const AUTHOR: &'static str = "";
    const ORGANIZATION: &'static str = "";
    const CONTACT: &'static str = "";
    const DISTRO: &'static str = "";
}

#[derive(Serialize, Deserialize, Debug)]
struct BehaviorInfo {
    name: String,
    brief_description: String,
    // NaiveDate supports Serde out of the box, but uses RFC3339 format. Provide
    // some custom logic to make it use our desired format.
    #[serde(with = "my_date_format")]
    creation_date: Option<NaiveDate>,
    description: Option<String>,
    url: Option<String>,
    license: Option<String>,
    module: Option<String>,
    author: Option<String>,
    organization: Option<String>,
    contact: Option<String>,
    distro: Option<String>,

    members: Vec<ObjectInfo>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct ObjectInfo {
    name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    description: String,
    #[serde(rename = "type")]
    type_info: TypeInfo,
    // Set to `Some(true)` if this ObjectInfo can be duplicated. If the
    // `allow_duplicate` field is not set or it is set to false, this will
    // display an error if this object appears multiple times
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_duplicates: Option<bool>,
    // Set to `Some(true)` if this ObjectInfo is required. If the `required`
    // field is not set or it is set to false
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deprecated: Option<bool>,
    #[serde(skip_serializing_if = "String::is_empty")]
    example: String,
}

impl ObjectInfo {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum TypeInfo {
    Boolean,
    Integer { min: Option<i64>, max: Option<i64> },
    Float { min: Option<f64>, max: Option<f64> },
    String { values: Vec<(String, String)> },
    // Comma separated list of unmanned values
    Tuple { values: Vec<ObjectInfo> },
    // Comma separated list of named fields
    Object { members: Vec<ObjectInfo> },
    // Colon separated list of unnamed values
    List { type_info: Box<TypeInfo> },
}

impl Default for TypeInfo {
    fn default() -> Self {
        Self::String { values: Vec::new() }
    }
}

// https://serde.rs/custom-date-format.html
mod my_date_format {
    use chrono::{NaiveDate, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%Y-%m-%d";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &Option<NaiveDate>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(date) = date {
            let s = format!("{}", date.format(FORMAT));
            serializer.serialize_str(&s)
        } else {
            serializer.serialize_none()
        }
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)?;
        Ok(Some(dt))
    }
}

#[cfg(test)]
mod test {
    use chrono::{NaiveDate, Utc};

    use crate::trees::manifest::{AppInfo, AppInfoTrait, ManifestItemType, ObjectInfo, TypeInfo};

    #[test]
    fn test_app_info_trait() {
        struct TestApp;

        impl AppInfoTrait for TestApp {
            const NAME: &'static str = "My Name";
            const CREATION_DATE: Option<NaiveDate> = NaiveDate::from_ymd_opt(2021, 3, 21);
            const BRIEF_DESCRIPTION: &'static str = "Test Application Brief Description";
            const MANIFEST_TYPE: crate::trees::manifest::ManifestItemType =
                ManifestItemType::Application;
        }

        struct TestApp2;

        impl AppInfoTrait for TestApp2 {
            const NAME: &'static str = "My Name";
            const CREATION_DATE: Option<NaiveDate> = NaiveDate::from_ymd_opt(2021, 3, 21);
        }

        println!(
            "TestApp brief_description: {:?}",
            TestApp::BRIEF_DESCRIPTION
        );

        println!(
            "TestApp2 brief_description: {:?}",
            TestApp2::BRIEF_DESCRIPTION
        );
    }

    #[test]
    fn test_app_info_struct() {
        use serde_yaml;
        struct TestApp;

        impl AppInfoTrait for TestApp {
            const NAME: &'static str = "My Name";
            const CREATION_DATE: Option<NaiveDate> = NaiveDate::from_ymd_opt(2021, 3, 21);
            const BRIEF_DESCRIPTION: &'static str = "Test Application Brief Description";
            const MANIFEST_TYPE: crate::trees::manifest::ManifestItemType =
                ManifestItemType::Application;
            // TODO: Cannot create a const vector. Should we convert that into an array?
            //const MISSION_MEMBERS: Vec<ObjectInfo> = vec![start_position];
        }

        type Latitude = f64;
        impl Into<TypeInfo> for Latitude {
            fn into(self) -> TypeInfo {
                TypeInfo::Float {
                    min: Some(-90.0),
                    max: Some(-90.0),
                }
            }
        }

        let mut test_app_info = AppInfo::new_from_trait::<TestApp>();

        let start_position = ObjectInfo::new("start_position");

        test_app_info.mission_members.push(start_position);

        println!(
            "AppInfo for TestApp: \n{}",
            serde_yaml::to_string(&test_app_info).unwrap()
        );
    }
}
