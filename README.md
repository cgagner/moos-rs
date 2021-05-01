# MOOS Client library for Rust

The Mission Oriented Operating Suite (MOOS) is a light weight, easy to use 
middleware for robots. 

More information about MOOS can be found 
[here](https://sites.google.com/site/moossoftware/). 

Tasks: 
- [x] Message struct
- [x] Message decode
- [x] Message encode
- [ ] ~~Packet struct~~ The Packet structure is no longer needed. 
- [x] Packet encode
- [x] Packet decode
- [ ] Async TCP client
    - [x] Connect
    - [x] Handshake
    - [x] Start write loop
    - [x] Start read loop
    - [x] Publish (Basic)
    - [x] Subscribe (Basic)
    - [x] Unsubscribe (Basic)
    - [x] On Subscribe, var name cannot be empty
    - [ ] Store/clear list of subscribed variables.
    - [x] Subscribe wildcard (AppPattern=*,VarPattern=*,Interval=0.0)
    - [x] Unsubscribe wildcard
    - [x] On Connect Callback
    - [ ] On Disconnect Callback
    - [ ] Add a close method to the client
    - [ ] Subscribe with fitler
    - [ ] Subscribe with callback
    - [ ] Publish with automatic type conversions
    - [x] Need to fix the latency timing reported in the DB_QOS message.
- [ ] MOOS Application struct or macro
    - [ ] Parse mission file
    - [ ] Parse arguments
    - [ ] Handle publishing App status (e.g. CPU load)
- [ ] Example / Tutorial



## Example MOOS App
The example-moos-app is an application used to test the various parts of
the client API. At the moment, it shouldn't be used as an example.

### Client-1
```bash
cargo run -- --moos_name=umm-1
```

### Client-2
```bash
cargo run -- --moos_name=umm-2 -s=TEST_12
```


# Credits

## Images
- [Moose Icon](https://pixabay.com/vectors/deer-mammal-moose-antler-animal-159022/): Image by <a href="https://pixabay.com/users/openclipart-vectors-30363/?utm_source=link-attribution&amp;utm_medium=referral&amp;utm_campaign=image&amp;utm_content=159022">OpenClipart-Vectors</a> from <a href="https://pixabay.com/?utm_source=link-attribution&amp;utm_medium=referral&amp;utm_campaign=image&amp;utm_content=159022">Pixabay</a>



