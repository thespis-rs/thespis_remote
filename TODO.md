# thespis_remote TODO


- expects and unwraps

- remove logging from peer and return events through pharos. Clients can log if and how they want to.
- move remaining logging to tracing?
- TODO's and FIXME's

## API

- we removed the possibility to add/remove services. Think about user scenarios. Should it be possible
  to add/remove entire service maps? After login for example?
	- we could possibly provide update_handler for services, but not insert handler on a shared reference.

- get rid of Send and Sync where possible


## Implementation

- further clean up service maps:
  - can we move things out of macro?

- benchmark
- zero copy networking: https://github.com/tokio-rs/bytes/pull/371
  would get rid of dependencies on tokio-codec and futures-codec.
- zero cost serialization. Kompakt?
  cheaper serialization: bincode as a feature flag?
- do we really want to use Bytes as underlying storage and use codec?


## Testing

- polish testing code. Abstract out things more so actual tests have less code.
- test all the error handling.
	- verify and document what events actually get sent to pharos. Currently nothing that happens in spawned tasks like timeouts.

- spawn a task that logs every event from pharos in common.
- tests + benchmarks for flow control and back pressure. Shouldn't we be able to achieve backpressure solely by bounded channels instead of having to add a special backpressure type?

- test system limits/memory consumption in relation to backpressure settings/max open connections...

- https://en.wikipedia.org/wiki/Slowloris_(computer_security)

- test supervised actors and automatic reconnect? (resilliance, start a new actor or reconnect to a backend and try again before reporting error to client?)

- does it work to communicate with code in webworkers in wasm?
- verify stability of UniqueID and generate one from a different programming language to be sure.

- fuzz
- CI, look at the futures .travis.yml for better configuration
- single threaded testing.


## Examples

- fix examples

- make a test communicating with a program in a different programming language.


# Docs

- documentation
- user guide

