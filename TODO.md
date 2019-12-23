# thespis_remote_impl TODO


- design: try to simplify

	- rethink macro, public interface, try to move things out of macro and review the need for
	  Service, ServiceMap, ServiceProvider, UniqueID
	- move error to impl
	-


	- remove the interface and all generics!
	- have a codec for both tokio and futures-codec
	- ideally take AsyncRead+AsyncWrite rather than Stream+Sink eg. make a method for tokio and one for futures version, but keep it stored as a Stream+Sink to simplify peer
	- have futures-codec and tokio optional

- get rid of async_runtime
- move ServicesRecipient out of macro
