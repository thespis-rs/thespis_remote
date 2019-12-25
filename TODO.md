# thespis_remote_impl TODO


- design: try to simplify

	- rethink macro, public interface, try to move things out of macro and review the need for
	  Service, ServiceMap

	  requirements:

	  - convenient way to define which services are exposed by which connection
	  - possibility to expose services defined  in another crate (crate that has just actor messages + Serialize, but not a service map)
	  - get an sid out of a service?
	  - sid is unique from service+namespace, but should be identical if compiled in different process


	- get rid of futures and tokio 0.1


	- tokio support

- get rid of async_runtime

- TODO's and FIXME's
- add tests
- fix examples
- documentation
