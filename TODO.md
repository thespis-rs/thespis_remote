# thespis_remote_impl TODO


- structured concurrency! All of these requests (at least calls) should stop when the connection
 closes. It makes no sense to process requests if we can't send a response anymore. Right now
 we have orphaned tasks that will keep running and keep processing.


- Think of all the scenarios and error handling/reporting -> still based on our older not so flexible model.
- evaluate all places where peer can block while processing messages and document/test.
  - the opposite, flow control and back pressure?
    - tests + benchmarks for flow control and back pressure.

- Send, Call and Broadcast?

- TODO's and FIXME's
- get rid of Send and Sync where possible
- add tests and comments for everything (especially error handling.)
- supervised actors and automatic reconnect? (resilliance, start a new actor or reconnect to a backend and try again before reporting error to client?)

- fix examples
- verify stability of UniqueID and generate one from a different programming language to be sure.
- benchmark
- documentation
- user guide

- find a way to separate logs per task
- publish async_executors
- abstract channels (can pharos benefit as well)
- cleanup thespis and thespis_impl
