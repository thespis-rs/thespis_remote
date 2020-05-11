# thespis_remote TODO



## API

- simplify UniqueID, just a u64?

- move remaining logging to tracing? And consider re-adding errors as logs. The point of logging is that
  we can get nice side by side logs of each component if we want, which is not so much the case if the
  user has to log stuff coming in over pharos. I think pharos should be for events you want to react to
  programatorically, not for logging.

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


### Test automation

- injection points. Places where we can easily inject input:
  - the network boundary: we can have a certain setup peer that exposes services, bombard it with all sorts of network packages and examine it's behavior. Same on the client. A standard client that does some standard requests. Send it all sorts of answers and measure that it behaves correctly.
    This can be finalized with fuzz testing.

  - mock the wire format -> allows testing behavior of peer.
  - mock the service map -> allows testing behavior of peer.

  - mock the peer -> allows testing the service map implementation.

  - mock the handlers? If they panic, time out, ... what happens.

## Examples

- fix examples

- make a test communicating with a program in a different programming language.


# Docs

- documentation
- user guide

- TODO's and FIXME's
- expects and unwraps
