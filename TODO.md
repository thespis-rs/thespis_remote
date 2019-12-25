# thespis_remote_impl TODO


- design: try to simplify

	- rename MultiServiceImpl, MulServTokioCodec

	- rethink macro, public interface, try to move things out of macro and review the need for
	  Service, ServiceMap, ServiceProvider, UniqueID

	  requirements:

	  - convenient way to define which services are exposed by which connection
	  - possibility to expose services defined  in another crate (crate that has just actor messages + Serialize, but not a service map)
	  - get an sid out of a service?
	  - sid is unique from service+namespace, but should be identical if compiled in different process


	- get rid of futures and tokio 0.1


	- have a codec for both tokio and futures-codec
	- ideally take AsyncRead+AsyncWrite rather than Stream+Sink eg. make a method for tokio and one for futures version, but keep it stored as a Stream+Sink to simplify peer
	- have futures-codec and tokio optional

- get rid of async_runtime
