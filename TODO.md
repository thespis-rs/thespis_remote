# thespis_remote_impl TODO


- design: try to simplify

	- get rid of futures 0.1
	- tokio support

- structured concurrency! All of these requests (at least calls) should stop when the connection
 closes. It makes no sense to process requests if we can't send a response anymore. Right now
 we have orphaned tasks that will keep running and keep processing.


- take a closure in register_relayed_services so people can implement load balancing.
- make sid's static references everywhere
- evaluate all places where peer can block while processing messages and document.
- allow stopping actors?
- allow replacing handlers?
- do not depend on any thespis implementation, only on interface, so user can
  choose implementation (only matters for our public interface).
- get rid of Send and Sync where possible
- supervised actors and automatic reconnect?

- TODO's and FIXME's
- add tests and comments for everything (especially error handling.)
- fix examples
- verify stability of UniqueID and generate one from a different programming language to be sure.
- documentation
- user guide

- find a way to separate logs per task
