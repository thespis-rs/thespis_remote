# thespis_impl_remote
The reference implementation of the thespis remote actor model


## TODO

- let ServiceMap have a debug implementation which will print out all services with their unique id, so it can be put in the documentation of programs and libraries. Peer should probably also be able to tell the remote which services it provides.
- test everything

- we don't close the connection when errors happen in the spawned tasks in send_service and call_service in the macro... bad! It also won't emit events for them...bad again!
- client code for remote actors is not generic, it will only work on MultiServiceImpl
- remote should store and resend messages for call if we don't get an acknowledgement? If ever you receive twice, you should drop it? Does tcp not guarantee arrival here? What with connection loss? The concept is best efforts to deliver a message at most once.
- write benchmarks for remote actors
- remote Addr? if the actor is known compile time?

## Remote design
