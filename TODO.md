# thespis_remote TODO

- currently errors are returned to the remote even for Send. Should the nursery_stream or Handler<RequestError>
  filter these out? Or should service maps not return them at all?

- remove logging from peer and return events through pharos. Clients can log if and how they want to.

- verify all sends to self so they are spawned, not awaited. Create tests to show the deadlock.

- how to do a relay without compiling in the types?

- we could possibly provide update_handler for services, but not insert handler on a shared reference.

- update to new tokio codec version

- eleminate the need for passing peer addr to service map where we can

- we removed the possibility to add/remove services. Think about user scenarios. Should it be possible
  to add/remove entire service maps? After login for example?

- verify and document what events actually get sent to pharos. Currently nothing that happens in spawned tasks like timeouts.

- further clean up service maps:
  - can we move things out of macro?
  - usage with HashMany and complicated code in incoming

- do we really want to use Bytes as underlying storage and use codec?
- should UniqueID and ServiceID be Copy?
- Kompakt?
- https://en.wikipedia.org/wiki/Slowloris_(computer_security)


- All of these requests (at least calls) should stop when the connection
 closes. It makes no sense to process requests if we can't send a response anymore. Right now
 we have orphaned tasks that will keep running and keep processing. The solution to this is structured con
 currency, having a nursery and spawning both the mailbox of peer on it as well as giving a reference to
 the nursery to peer means that all these tasks would be tied to the lifetime of the nursery.


- tests + benchmarks for flow control and back pressure. Shouldn't we be able to achieve backpressure solely by bounded channels instead of having to add a special backpressure type?

- fairness and starvation

- does it work to communicate with code in webworkers in wasm?
- TODO's and FIXME's
- test supervised actors and automatic reconnect? (resilliance, start a new actor or reconnect to a backend and try again before reporting error to client?)
- get rid of Send and Sync where possible
- add tests and comments for everything (especially error handling.)
- fuzz

- fix examples
- verify stability of UniqueID and generate one from a different programming language to be sure.
- make a test communicating with a program in a different programming language.
- benchmark
- test system limits/memory consumption in relation to backpressure settings/max open connections...
- documentation
- user guide

- find a way to separate logs per task
- publish async_executors
- abstract channels (can pharos benefit as well)
- cleanup thespis and thespis_impl
- CI, look at the futures .travis.json for better configuration
