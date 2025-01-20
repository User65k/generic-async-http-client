[![Project Status: Active – The project has reached a stable, usable state and is being actively developed.](https://www.repostatus.org/badges/latest/active.svg)](https://www.repostatus.org/#active)
[![Crates.io][crates-badge]][crates-url]
[![Released API docs](https://docs.rs/generic-async-http-client/badge.svg)](https://docs.rs/generic-async-http-client)
[![MIT licensed][mit-badge]][mit-url]

[crates-badge]: https://img.shields.io/crates/v/generic-async-http-client.svg
[crates-url]: https://crates.io/crates/generic-async-http-client
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/User65k/generic-async-http-client/blob/master/LICENSE

A generic async HTTP request create.

It is meant to be a thin wrapper around various HTTP clients
and handles TLS, serialization and parsing.

The main goal is to allow binaries (that pull in some libraries that make use of a HTTP client)
to **specify what implementation should be used**.

And if there is a **Proxy**. If not specified auto detection is performed by looking at `HTTP_PROXY`.

# Features
You need to specify via features what crates are used to do the actual work.

|feature flag|Meaning|
|---|---|
|use_hyper|Use [hyper](https://crates.io/crates/hyper) for HTTP ![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/User65k/generic-async-http-client/test_hyper_rustls.yml)|
|use_async_h1|Use [async_h1](https://crates.io/crates/async_h1) for HTTP ![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/User65k/generic-async-http-client/test_async_std_rustls.yml)|
|rustls_byoc|Add support for HTTPS via [rustls](https://crates.io/crates/rustls) - **You** need to set a [CryptoProvider](https://docs.rs/rustls/latest/rustls/crypto/struct.CryptoProvider.html#using-the-per-process-default-cryptoprovider)|
|rustls_ring|Like `rustls_byoc` but use [ring](https://crates.io/crates/ring) for crypto|
|rustls_aws_lc_rs|Like `rustls_byoc` but use [aws-lc-rs](https://crates.io/crates/aws-lc-rs) for crypto|
|rustls_fips|Like `rustls_byoc` but use [aws-lc-rs](https://crates.io/crates/aws-lc-rs) in FIPS mode for crypto|
|proxies|Add support for Socks5 and HTTP proxy|
|hyper_native_tls|Use [hyper](https://crates.io/crates/hyper) for HTTP and do HTTPS via [native_tls](https://crates.io/crates/native_tls)  ![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/User65k/generic-async-http-client/test_hyper_nativetls.yml)|
|async_native_tls|Use [async_h1](https://crates.io/crates/async_h1) for HTTP and do HTTPS via [native_tls](https://crates.io/crates/native_tls)  ![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/User65k/generic-async-http-client/test_async_std_nativetls.yml)|
|http2|Use http2 if available (only works with `use_hyper`)|

Without anything specified you will end up with *No HTTP backend was selected*.
If you use this crate for a library, please [reexport](https://doc.rust-lang.org/cargo/reference/features.html#dependency-features) the appropriate features.

# Motivation

Rust offers different async runtimes that - on a high level - offer the same thing: asynchronous functions for files, sockets and so on.

So if you write a lib and need some basic stuff (like an http client) you sometimes have to make choices that are not what your crates users would have liked.
For example:
I wrote a [webserver](https://github.com/User65k/flash_rust_ws) based on hyper and wanted to add ACME.
A crate I found did what I needed but used async-h1 and async-std. While that worked, it did increase the binary size and crates I depend on by a good amount.

So I wrote this. You can specify which backend to use.
In the Webserver case, using tokio which is already a dependency VS async-std did lead to 81 less crates and a 350kB smaller binary.
Using (and [async-acme](https://crates.io/crates/async-acme)):
```toml
[profile.release]
lto = "fat"
codegen-units = 1
```

Also for http clients: there should be a way to add a proxy for all libs that use it.

# Plans

2. Add Sessions - to make multiple requests more efficient
3. Add a cookie jar for the sessions
4. Allow a Body to be streamed from a server
5. Allow a Body to be streamed to a server
