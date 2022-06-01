# thespis_remote TODO

- test talking to a specific actor by using const generics on the service type.

- test grace_period

- examples for all feature, backpressure, pubsub, ...

- docs in service_map_macro are out of date, still mention recipient...

- can incoming messages provide unfair competition for Peer mailbox which will prevent outgoing? eg. can it continuously fill the mailbox preventing outgoing messages from getting out? With default back pressure?

  - It will depend on the fairness of the channel. When the outgoing blocks, it should get a slot.

- PeerErr is now also thrown by other objects, notably PubSub and RelayMap. Verify consistency,
  as these do not have a peer_id.
- in general verify and test all error handling.

- figure out a way to let the user know which connection a message came from to make it easier to do authorization, as opposed to sending a secret along with every message.

- reconnect strategy? Allow people to give the peer a new connection without data loss?

    - returning the peer object and spawning the mailbox again, but that requires the signature of mailbox to change so the actor can be returned as well as the mailbox.
    - take connection as an object that has shared mutability so the connection can be reset?

  what about server vs client side? Client could reconnect, but on server you need to wait for the client to reconnect and then
  instead of making a new peer, you need to identify that this is a reconnect and recover the peer?
  
  
## from readme

  - switch to futures_ringbuf for testing and examples

  - fuzz testing

  - WASM in tests
  - Peer should probably be able to tell the remote which services it provides.
  - we don't close the connection when errors happen in the spawned tasks in send_service and call_service in the macro... bad! It also won't emit events for them...bad again!
  - remote should store and resend messages for call if we don't get an acknowledgement? If ever you receive twice, you should drop it? Does tcp not guarantee arrival here? What with connection loss? The concept is best efforts to deliver a message at most once.
  - write benchmarks for remote actors

  ## Remote design

  - currently relaying is very static. A unique service id exists only at compile time. Imagine a chat service where many identical clients connect to a central server. We could not currently use the relay feature to relay messages from users to eachother, because they would each need to compile a different binary... to be solved.



## API

- move remaining logging to tracing? And consider re-adding errors as logs. The point of logging is that
  we can get nice side by side logs of each component if we want, which is not so much the case if the
  user has to log stuff coming in over pharos. I think pharos should be for events you want to react to
  programatorically, not for logging.

- we removed the possibility to add/remove services. Think about user scenarios. Should it be possible
  to add/remove entire service maps? After login for example?
	- we could possibly provide update_handler for services, but not insert handler on a shared reference.

- get rid of Send and Sync bounds where possible

- bring back tokio support


## Implementation

- further clean up service maps:
  - can we move things out of macro?

- benchmark
- zero copy networking: https://github.com/tokio-rs/bytes/pull/371
  would get rid of dependencies on tokio-codec and futures-codec.
- zero cost serialization. Kompakt?
  cheaper serialization: bincode as a feature flag?
- do we really want to use Bytes as underlying storage and use codec?

- The wire format is a hand baked solution just to get it working. Now we should find out what the final formats might look like. Cap'n proto? or SBE? : https://polysync.io/blog/session-types-for-hearty-codecs


## Testing

- encoder/decoder, add missing tests.
- figure out the io errors. Can be returned by the decoder, eg. connection closed by remote vs connection loss... can we detect the difference?

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


## Benchmarking

  - compare BytesFormat and ThesWF
  - check whether we allocate buffers of the correct size. We allocate mem::size_of unserialized message. If CBOR where on average just a bit bigger than
    the in memory representation, we should default to allocating bigger. and document encoder and decoder properly.
  - non initialized buffers?

## Examples

- fix examples

- make a test communicating with a program in a different programming language.


# Docs

- documentation
- user guide

- TODO's and FIXME's
- expects and unwraps
