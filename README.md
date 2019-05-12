# thespis_impl_remote
The reference implementation of the thespis remote actor model


## TODO

- we don't close the connection when errors happen in the spawned tasks in send_service and call_service in the macro... bad! It also won't emit events for them...bad again!
- can we move the service recipient out of the macro? Like have a generic method on peer that will return
- we could make a more ergonomic api on peer if we store handlers in service map. It would move code from peer to the macro, but it would allow something like `let (peer_addr, peer_events) = listen_tcp( "127.0.0.1:4343", vec![ service_map1, service_map2 ] );` which is currently impossible, because you need to call register_service on peer with hardcoded types. Service map won't be a zero sized type anymore, but we wouldn't have to keep a bunch of them like we do right now. We could store them consistently like relays, sid -> sm_id, sm_id -> service map. We could even maybe create greater consistency by creating a relay map. It would move responsibility into those objects, lightening the impl of peer.
- client code for remote actors is not generic, it will only work on MultiServiceImpl
- let ServiceMap have a debug implementation which will print out all services with their unique id, so it can be put
  in the documentation of programs and libraries. Peer should probably also be able to tell the remote which services
  it provides.
  you a recipient if the peer can reach that service?
- compile time verification that a relay actually relays the sids we are given in a peer?

- remote should store and resend messages for call if we don't get an acknowledgement? If ever you receive twice, you should drop it? Does tcp not guarantee arrival here? What with connection loss? The concept is best efforts to deliver a message at most once.
- write benchmarks for remote actors
- remote Addr? if the actor is known compile time?

## Remote design
