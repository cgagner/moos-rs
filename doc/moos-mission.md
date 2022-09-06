# MOOS Mission

## Mission Format

***TODO:***

* [ ] Figure out what square brackets are used for

### General

MOOS uses a custom file format that is used to configure applications. The
file is a line-based, human-readable format.

Comments that start with a double slash (`//`). Comments can be at the
start of the line or after other expressions. There is **NOT** a way to 
specify multi-line comments. 

Global values are set as name-value pairs. E.G.:

```moos
MOOSTimeWarp = 10 // Global Time Warp to be used by all applications 
```

By convention, global variables should be placed at the top of the file, 
though this is not strictly enforced. 

Application specific configuration values are located in a `ProcessConfig`
block. Below is an example configuration block for the `uProcessWatch` 
application:

```moos
//------------------------------------------
// uProcessWatch

ProcessConfig = uProcessWatch
{
  AppTick   = 4
  CommsTick = 4

  watch_all = true
  nowatch   = uPokeDB*
  nowatch   = uXMS*
}
```

In the example above, `AppTick`, `CommsTick`, `watch_all`, and `nowatch` are
application configuration values. It should be noted that `nowatch` is
repeated. The interpretation of repeated values is application specific. In 
some cases, repeated values are used additive (I.E. values are combined). In
other cases, only the first value. The behavior is determined by the API
method called to fetch the value. See [C++ API Notes](#c-api-notes). In
general, this is because application can fetch values by name or they can
iterate over all values. 

### Variables

Variable substitution is permitted using the `${VAR}` syntax, 
where `VAR` is an example variable. Variables can be defined inside the 
mission file using a `define: <name>=<value>` statement, where `<name>` is
the variable name and `<value>` is the variable value. Variables that are
not defined locally are assumed to be environment variables. Below is a
simple example:

```moos
define: APPTICK=1.0

ProcessConfig = someApplication
{
  AppTick=${APPTICK} // Set using the define above
  Home=${HOME} // Set using the `HOME` environment variable
}
```

### Vectors and Matrices

Most often values are booleans, doubles, integers, or strings. However, vectors
or matrices (flattened into a vector) can be used by using the format 
`[NxM]{a,b,c,...}`, where `N` is the number of rows,  `M` is the number of 
columns and `a,b,c...` are the values. For short, the columns can be omitted:

```moos
MyVector  = [5]{0,1,2,3,4} // Same ase [5x1]{0,1,2,3,4}
```

Specifying the incorrect number of columns or rows will result in a failure. 

### Square Brackets

The mission reader has two methods for iterating through name value pairs in
an application configuration:

* `bool CProcessConfigReader::GetConfigurationAndPreserveSpace(std::string sAppName, STRING_LIST &Params)`
* `bool CProcessConfigReader::GetConfiguration(std::string sAppName, STRING_LIST &Params)`

Each of these methods populate a list of strings that are in the `name=value` 
format. However, if the line does not have a value and it has a square bracket
(either `[` or  `]`), the line is added to the list of strings that is
returned. This appears to be an undocumented feature. It potentially could be
used for dividing an applications configuration block into different
sections. E.G.:

```moos
ProcessConfig = someApplication
{
  [general]
  debug = false

  [server]
  debug = true
}
```



## C++ API Notes

The `CMOOSApp` has a member variable `m_MissionReader` that is an instance of 
a `CProcessConfigReader` class. The `CMOOSApp` initializes the mission reader
in the `Run` method by calling the `SetAppName` method. 

To fetch global values, the mission reader has a collection `GetValue`
methods. For example, the following will get the `MOOSTimeWarp` global
value from the mission file:

```c++
double dfTimeWarp = 1.0;
if(m_MissionReader.GetValue("MOOSTimeWarp", dfTimeWarp))
{
    // Do something with dfTimeWarp
}
```

To fetch application specific configuration values, the mission reader has
a collection of `GetConfigurationParam` methods. For example, the following
will get the `APPTICK` value for the current `CMOOSApp`:

```c++
double dfFreq = 1.0;
if(m_MissionReader.GetConfigurationParam("APPTICK",dfFreq))
{
    // Do something with dfFreq
}
```

**NOTE:** The `GetConfigurationParam` methods rely on the application name
that is set in the `CProcessConfigReader::SetAppName` method, which is called
from `CMOOSApp::Run`. There are versions that allow users to pass in the
application name, but most applications use the former. 

The `CMOOSFileReader::GetNextValidLine(bool bDoSubstitution=true)` method
is called when parsing configuration paramters, global variables, and while
iterating. The method will skip empty lines and remove comments from the lines.
If `bDoSubstitution` is `true` (the default), strings with variables 
(E.G `${VAR}`) will be replaced with their defined values.

***WARNING:*** Variables that contain a double slash (`//`) will **NOT** be 
treated as a comment. 

The mission reader will enable verbatim quoting by default. This will treat 
quoted strings as literals. The quotation marks will be removed from said 
literals. Additionally, double slashes (`//`) inside of quotes will **NOT**
be treated as comments (E.G. "Hello//World"). This can be disabled by calling
the `CMOOSFileReader::EnableVerbatimQuoting` method.

### Searched Parameters

The mission reader keeps track of all of the configuration paramters
searched for by each application name. This allows the `CMOOSApp` to print
all of the searched configuration file parameters. This can be printed out by
passing the `--moos_configuration_audit` command-line option to a MOOS
application. See `CMOOSApp::PrintSearchedConfigurationFileParameters()`.

## Rust API Notes

### Parser

We are currently using asyncronous programming. As such, we should probably
use async/await when reading the mission files. However, I don't want to 
tie the implementaiton to tokio. We'll need to separate the reading from
the parsing so we can have both an async and sync version.

TODO:

* Enable spell check on VS Code.
* Create a method that strips comments off of lines
* Create a method for performing variable substitutions.
* Need to handle async read




### Backing Value Object

After parsing the mission, it'd be nice to provide an easy way to access the
data. I like the way that JSON data is accessed using the Value class:

```rust
use serde_json::{Result, Value};

fn untyped_example() -> Result<()> {
    // Some JSON input data as a &str. Maybe this comes from the user.
    let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        }"#;

    // Parse the string of data into serde_json::Value.
    let v: Value = serde_json::from_str(data)?;

    // Access parts of the data by indexing with square brackets.
    println!("Please call {} at the number {}", v["name"], v["phones"][0]);

    Ok(())
}
```

Data inside of the Value object `v` can be accessed using square brackets.
For example, the first phone number can be accessed using `v["phones"][0]`.
The value object has methods for checking the type and returning the value as
that type. Using this type of interface for MOOS would like like:

```rust
pub fn handle_mission(&mission: serde_json::Value) -> bool {
  // Get the time warp or default to 1.0
  let time_warp = v["MOOSTimeWarp"].as_f64().unwrap_or(1.0);
  // Get the app_tick for MyApp or default to 1.0
  let app_tick = v["MyApp"]["AppTick"].as_f64().unwrap_or(1.0);
  // Get the log_dir for MyApp or panic because it is missing. NOTE: You 
  // shouldn't really panic because of a missing log_dir. This is just an 
  // example.
  let log_dir = v["MyApp"]["log_dir"]
      .as_str()
      .expect("MyApp requires the log_dir configuration parameter!");
}
```

My first thought was to implement a custom Value class for MOOS. However,
it gets pretty complicated when get beyond primitive types (E.G. arrays and 
maps).

There are several versions of the `Value` class available in the `serde-json`, 
`serde-yaml`, and `toml` crates. Each have their pro's and cons:

#### Serde YAML Value

Pros: 

* Simple API

Cons: 

* Repository is owned by a single user (not a group). It has 33 contributors, 
but no official backing. 
* Depends on `rust-yaml`, which the build is failing on the Nightly Rust build. 


#### TOML Value

Pros: 

* Simple API
* Used by Cargo (the official Rust Package manager and build system)

Cons:

* Doesn't support the empty/null type.
* Panics if a key is not found in a map.

#### Serde JSON Value

Pros:

* Officially supported by the Rust Serde group (89 contributors).
* Simple API

Cons: 

* JSON doesn't support comments. While this is not a big deal now, I am 
considering an alternative MOOS mission format that has validators
available. Comments are essential in mission files so JSON is off of the
table for as a potential format.