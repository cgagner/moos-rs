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
    - [x] Store list of published variables
    - [x] Subscribe wildcard (AppPattern=*,VarPattern=*,Interval=0.0)
    - [x] Unsubscribe wildcard
    - [x] On Connect Callback
    - [x] On Disconnect Callback
    - [ ] Call disconnect when there is a error on the socket.
    - [ ] Subscribe with fitler
    - [ ] Subscribe with callback
    - [ ] Publish with automatic type conversions
    - [x] Need to fix the latency timing reported in the DB_QOS message.
    - [ ] Update connect to reconnect if the the client is disconnected 
          from the db
    - [ ] Load in a configuration
        - [ ] Size of read/write buffers
        - [ ] Delay between reconnect attempts (default to 1)
    - [ ] Add atomic bool or conditional variable to stop the client
    - [ ] Add setter for is_time_correction_enabled
    - [ ] Handle the `SKEW_TOLERANCE` in `lib.rs`
    - [ ] Implement `Drop` for the `AsyncClient`
- [ ] MOOS Application struct or macro
    - [ ] Parse mission file
    - [ ] Parse arguments
    - [ ] Handle publishing App status (e.g. CPU load)
- [ ] Example / Tutorial
- [ ] Templates [cargo-generate](https://github.com/cargo-generate/cargo-generate)
    - [ ] Make a template for a Rust MOOS Application
    - [ ] Create a tutorial for using templates
    



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

# Notes

1. This project requires at least Rust 1.52. This is because of the addition 
   of the [str::split_once](https://doc.rust-lang.org/stable/std/primitive.str.html#method.split_once) 
   method, which is the Rust version of `MOOSChomp` from the original C++ 
   library.

# Credits

## Images
- [Moose Icon](https://pixabay.com/vectors/deer-mammal-moose-antler-animal-159022/): Image by <a href="https://pixabay.com/users/openclipart-vectors-30363/?utm_source=link-attribution&amp;utm_medium=referral&amp;utm_campaign=image&amp;utm_content=159022">OpenClipart-Vectors</a> from <a href="https://pixabay.com/?utm_source=link-attribution&amp;utm_medium=referral&amp;utm_campaign=image&amp;utm_content=159022">Pixabay</a>



