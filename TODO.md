# thespis_remote_impl TODO


- design: try to simplify

	- rethink macro, public interface, try to move things out of macro and review the need for
	  Service


	- get rid of futures 0.1


	- tokio support

- get rid of async_runtime
- rewrite peer incoming:
  - have an actor for handling each incoming request, since it has to run concurrently
    to the peer itself
  - probably it will be stateless, in which case it falls in the category of actors that
    can process multiple messages at the same time rather than one by one, so probably
    make an actor proxy, which is some kind of pool which will create new actors when needed.
    If it has no state, why not just spawn an async method instead of an actor?
  - structured concurrency! All of these requests (at least calls) should stop when the connection
    closes. It makes no sense to process requests if we can't send a response anymore. Right now
    we have orphaned tasks that will keep running and keep processing.

- do not depend on any thespis implementation, only on interface, so user can
  choose implementation (only matters for our public interface).
- get rid of Send and Sync where possible

- TODO's and FIXME's
- add tests
- fix examples
- documentation
