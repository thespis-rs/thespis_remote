# thespis_remote_impl TODO


- design: try to simplify

	- get rid of futures 0.1
	- tokio support

- rewrite peer incoming:
  - do not close stream when the actor message cannot be deserialized. only when the wireformat can't be deserialized.
  - structured concurrency! All of these requests (at least calls) should stop when the connection
    closes. It makes no sense to process requests if we can't send a response anymore. Right now
    we have orphaned tasks that will keep running and keep processing.


- make sid's static references everywhere
- keep people from relaying service id's they also provide in the local process
- evaluate all places where peer can block while processing messages and document.
- refactor incoming to make the methods shorter
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
- user guide

- find a way to separate logs per task
