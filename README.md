# thespis_remote
The reference implementation of the thespis remote actor model


## TODO

- service_map macro should not block on the processing of the handlers.
- if we implement send_after on thespis, we don't need an executor anymore in peer.

- why does handling an incoming send for a service require services to have the addr of the peer?
  We seem to use it to feed error information to the peer, but sends opt out of feedback normally.
  Only calls are guaranteed to get feedback related to the processing.

- switch to futures_ringbuf for testing and examples

- fuzz testing

- WASM in tests
- use futures 0.3 codecs instead of tokio
- Peer should probably be able to tell the remote which services it provides.
- we don't close the connection when errors happen in the spawned tasks in send_service and call_service in the macro... bad! It also won't emit events for them...bad again!
- client code for remote actors is not generic, it will only work on MultiServiceImpl
- remote should store and resend messages for call if we don't get an acknowledgement? If ever you receive twice, you should drop it? Does tcp not guarantee arrival here? What with connection loss? The concept is best efforts to deliver a message at most once.
- write benchmarks for remote actors
- remote Addr? if the actor is known compile time?

## Remote design

- currently relaying is very static. A unique service id exists only at compile time. Imagine a chat service where many identical clients connect to a central server. We could not currently use the relay feature to relay messages from users to eachother, because they would each need to compile a different binary... to be solved.
