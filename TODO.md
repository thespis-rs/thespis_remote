# thespis_remote_impl TODO


- design: try to simplify

	- rethink macro, public interface, try to move things out of macro and review the need for
	  Service


	- get rid of futures 0.1


	- tokio support

- rewrite peer incoming:
  - let ServiceID display as typename, and debug as both number and type name.
  - let call_service and send_service be async directly instead of returning a result to an async.
  - make sure all logging includes peer.identity()
  - have an actor for handling each incoming request, since it has to run concurrently
    to the peer itself
  - probably it will be stateless, in which case it falls in the category of actors that
    can process multiple messages at the same time rather than one by one, so probably
    make an actor proxy, which is some kind of pool which will create new actors when needed.
    If it has no state, why not just spawn an async method instead of an actor?
  - structured concurrency! All of these requests (at least calls) should stop when the connection
    closes. It makes no sense to process requests if we can't send a response anymore. Right now
    we have orphaned tasks that will keep running and keep processing.
  - have call_service and send_service be async methods that are spawned by peer, so the service_map
    can keep a copy of the peer address rather than cloning it on every incoming message.
    that means that when closing, we need to remove that address from the service map.

- evaluate all places where peer can block while processing messages and document.
- allow stopping actors?
- allow replacing handlers?
- do not depend on any thespis implementation, only on interface, so user can
  choose implementation (only matters for our public interface).
- get rid of Send and Sync where possible

- TODO's and FIXME's
- add tests
- fix examples
- verify stability of UniqueID and generate one from a different programming language to be sure.
- documentation

- find a way to separate logs per task
