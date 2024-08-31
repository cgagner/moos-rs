# MOOS-IvP Behavior Configuration File Format


Comments: Start with `//` to the end of the line.


## Variable Initialize

```
initialize <variable> = <value>
```


Multiple initializations may be on the same line by separating each pair with
a comma.

```
initialize <variable1> = <value1>, <variable2> = <value2> // Some Comment
```

MOOS Variables are case sensitive. Values may not be, but it depends on the
behavior. 

By default, `initialize` will post after the first helm iteration and will
overwrite values in the MOOSDB. You can use `initialize_` (with an underscore)
do defer the overwrite for another helm iteration if the variable hasn't
already been posted.

## Behavior Block

```
Behavior = <behavior-type>
{
  <parameter> = <value>
  <parameter> = <value>
}
```

* `Behavior` keyword is not case sensitive. 
* `<behavior-type>` is case sensitive. This is the class name for the behavior.
* The behavior declaration is "followed by an open brace on a separate line" 
  per the documentation. TODO: Need to verify this needs to be on a separate
  line.
* Parameters are separated into two groups: one group for the default
  parameters of an IvP Behavior, and another for behavior specific
  parameters. Convention is default params are at the top and left justified.
  Behavior specific params are typically right justified to equals sign.
* Some parameters are mandatory. However, this is not enforced and relies
  on documentation. We may want to handle that in the language server.
* The `behavior-type` can be reused. However, each behavior should have a
  unique `name`. 

## Hierarchical Mode Declarations


```
Set <mode-variable-name> = <mode-value>
{
<mode-variable-name> = <parent-value>
<condition>
. . .
<condition>
} <else-value>
```

* `Set` keyword is case insensitive.
* `<condition>` are handled the same as in behaviors.
* Mode declarations of children need to be listed after the declarations of 
  parents in the behavior file.

Example
```
Set MODE = Active {
  DEPLOY = true
} Inactive
Set MODE = Surveying {
  MODE = Active
  RETURN != true
} Returning
```

* `Active`
* `Active:Surveying`
* `Active:Returning`
* `Inactive`

In the more complicated example, "MODE=Alpha:Echo", is specified fully, i.e.,
"MODE=Echo" would not achieve the desired result.

We should add a check in the language server to verify modes are correct.

"Realizable" nodes are colored green in the tree documentation. Realizable
nodes may or may not have children.

```
Set MODE = Alpha {
  MISSION = SURVEYING
}
Set MODE = Bravo {
  MISSION = LOITERING
} Charlie
Set MODE = Delta {
  MODE = Alpha
  SITE = Archipelagos
} Echo
Set MODE = Foxtrot {
  MODE = Charlie
  VIDEO = Streaming
} Golf
Set Mode = Sierra {
  MODE = Alpha:Echo
  WATER_DEPTH = Shallow
}
Set Mode = Tango {
  MODE = Alpha:Echo
  WATER_DEPTH = Deep
}
```

TODO: Need to contact Mike about this example. Per the documentation:

"The <mode-variable-name>, <parent-value> and <else-value> are case sensitive"

