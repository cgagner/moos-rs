# MOOS COMMS Client

## Connection Details

### Client Loop (Synchronous)
1. While !quit
2. Create Socket 
3. Allocate buffers (128KB send, 128KB receive)
4. Connect to Server
5. Apply Recurrent Subscriptions
6. While !quit
7. Do Client Work
8. Sleep based on comms frequency.. 

### Connect To Server
1. Connect to Server
  a. If exception, increment attempt counter, sleep for 1 second, retry.
2. Handshake
  b. If success, mark connected=true, mark last connection time.
  c. Call connected callback.


### Handshake

Client: 
  1. Send "ELKS CAN'T DANCE 2/8/10". (char[32]) (null terminated) 
  2. Send MOOS Data message, key='asynchronous', data = m_sMyName.c_str().
  3. Read WelcomeMessage
    a. If WelcomeMessage.Type == MOOS_POISON, return false.
    b. Else, 
      1. Store the WelcomeMessage.GetCommunity()
      2. Check if WelcomeMessage.GetString() == 'asynchronous'
      3. Store WelcomeMessage.SourceAux: as DBHostAsSeenByDB. ('hostname')
      4. Store WelcomeMessage.GetDouble() as skew

### Messages
1. Terminate: MoosMessage(MOOS_TERMINATE_CONNECTION, "--quit", 0)
2. Timing: MoosMessage(MOOS_TIMING, "_async_timing", 0.0, MOOSLocalTime())
3. InitialHandshake: MoosMessage(MOOS_DATA, "asynchronous", MyName (string))
4. Unregister: MoosMessage(MOOS_UNREGISTER, vName, 0.0)
5. WildCardUnregister: MoosMessage(MOOS_WILDCARD_UNREGISTER, MyName, String=("AppPattern", "VarPattern", "Interval"))
6. Register: MoosMessage(MOOS_REGISTER, vName, interval)
7. WildCardRegister: MoosMessage(MOOS_WILDCARD_REGISTER, MyName, String=("AppPattern", "VarPattern", "Interval"))
8. Notify: MoosMessage(MOOS_NOTIFY, vName, <String|Double|Binary>, time)
9. ServerRequest: MoosMessage(MOOS_SERVER_REQUEST, what, "")



## Iterate Modes

|                     | REGULAR_ITERATE_AND_MAIL                              |
|---------------------|-------------------------------------------------------|
| Summary             | This mode is the default just as in pre-V10 releases ***Iterate()*** and ***OnNewMail()*** are called regularly and if mail is available, in lock step.|
| Configuration Block | IterateMode=0 |
| OnNewMail           | called at most every ***1/AppTick*** seconds. If mail has arrived ***OnNewMail()*** will be called just before ***Iterate()***  |
| Iterate             | called every ***1/AppTick*** seconds. So if AppTick=10 ***Iterate()*** will be called at 10Hz.    |
| Role of AppTick     | sets the speed of ***Iterate()*** in calls per second |
| Role of MaxAppTick  | not used                                              |
| Role of CommsTick   | not used as communications are asynchronous |


|                     | REGULAR_ITERATE_AND_MAIL                              |
|---------------------|-------------------------------------------------------|
| Summary             | The rate at which Iterate is called is coupled to the reception of mail. As soon as mail becomes available OnNewMail is called and is then followed by ***Iterate()***. If no mail arrives for ***1/AppTick*** seconds then iterate is called by itself. When mail is arriving ***Iterate()*** and ***OnNewMail()*** are synchronous - if ***OnNewMail()*** is called it will always be followed by a called to ***Iterate()***|
| Configuration Block | IterateMode=1 |
| OnNewMail           | Called at up to MaxAppTick times per second. So if ***MaxAppTick=100*** ***OnNewMail()*** will be called in response to the reception of new mail at up to ***100Hz***.|
| Iterate             | called at least ***AppTick*** times per second (if no mail) and up to ***MaxAppTick*** times per second |
| Role of AppTick     | sets a lower bound on the frequency at which ***Iterate()*** is called. So if ***AppTick = 10*** then Iterate will be called at at least ***10Hz*** |
| Role of MaxAppTick  | sets an upper limit on the rate at which Iterate (and ***OnNewMail***) can me called. If ***MaxAppTick=0*** both the speed is unlimited. |
| Role of CommsTick   | not used as communications are asynchronous |


|                     | REGULAR_ITERATE_AND_MAIL                              |
|---------------------|-------------------------------------------------------|
| Summary             | ***Iterate*** is called regularly and ***OnNewMail*** is called when new mail arrives. ***Iterate*** will not always be called after ***OnNewMail*** unless it is scheduled to do so. In this way ***OnNewMail*** and ***Iterate*** are decoupled. |
| Configuration Block | IterateMode=2 |
| OnNewMail           | Called as soon as mail is delivered at up to ***MaxAppTick*** times per second. |
| Iterate             | Called every ***AppTick*** times per second |
| Role of AppTick     | Sets the speed of ***Iterate()*** in calls per second as in ***REGULAR_ITERATE_AND_MAIL*** |
| Role of MaxAppTick  | Limits the rate at which ***OnNewMail*** is called. If ***MaxAppTick=0*** both the speed is unlimited. With a slight abuse of notation in this mode ***MaxAppTick*** does not control ***Iterate()*** speed at all - it simply limits the rate at which new mail can be responded to |
| Role of CommsTick   | not used as communications are asynchronous |