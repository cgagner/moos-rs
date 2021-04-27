# MOOS Client library for Rust

The Missino Oriented Operating Suite (MOOS) is a light weight, easy to use 
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
    - [ ] On Connect Callback
    - [ ] On Disconnect Callback
    - [ ] Subscribe with fitler
    - [ ] Subscribe with callback
    - [ ] Publish with automatic type conversions
- [ ] MOOS Application struct or macro
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





