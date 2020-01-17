# thespis_remote_impl TODO


- structured concurrency! All of these requests (at least calls) should stop when the connection
 closes. It makes no sense to process requests if we can't send a response anymore. Right now
 we have orphaned tasks that will keep running and keep processing.


- allow both closures and fixed addresses, in both relay_map and service_map_macro.

- evaluate all places where peer can block while processing messages and document/test.
- TODO's and FIXME's
- add tests and comments for everything (especially error handling.)
- get rid of Send and Sync where possible
- supervised actors and automatic reconnect?

- fix examples
- verify stability of UniqueID and generate one from a different programming language to be sure.
- documentation
- user guide

- find a way to separate logs per task
- publish async_executors
- abstract channels (can pharos benefit as well)
- cleanup thespis and thespis_impl
