A generic async HTTP request create.

It is meant to be a thin wrapper around various HTTP clients
and handles TLS, serialisation and parsing.

The main goal is to allow binaries (that pull in some libraries that make use of a HTTP client)
to specify what implementation should be used.

And if there is a Proxy. If not specified auto detection is performed by looking at HTTP_PROXY.

You need to specify via features what crates are used to the actual work.
- hyper (and tokio)
- async-h1 (and async-std)