# hello-rs

Suite of Rust demos.

## hello-axum

Simple HTTP/JSON service demo built with [auxm](https://github.com/tokio-rs/axum).

### Running the service

```
RUST_LOG=hello_axum=debug,tower_http=debug \
  CONFIG_DIR=hello-axum/config \
  APP__CLIENT_TONIC_CLIENT__endpoint=http://localhost:90 \
  cargo run -p hello-axum --features proxy |\
  jq '{timestamp, level, target, message: .fields.message}'
```

## hello-tonic-server

Simple gPRC demo service built with [tonic](https://github.com/hyperium/tonic).

### Running the service

```
RUST_LOG=hello_tonic_server=debug,tower_http=debug \
  CONFIG_DIR=hello-tonic-server/config \
  APP__SERVER__PORT=90 \
  cargo run -p hello-tonic-server |\
  jq '{timestamp, level, target, message: .fields.message}'
```

To test the client use `grpcurl`:

```
grpcurl -plaintext -import-path proto -proto hello.proto -d '{ "name": "Max" }' 127.0.0.1:90 hello.Hello.SayHello
```

## hello-tonic-client

Simple gPRC demo client built with [tonic](https://github.com/hyperium/tonic).

### Running the client

```
CONFIG_DIR=hello-tonic-client/config \
  cargo run -p hello-tonic-client
```

## License ##

This code is open source software licensed under the [Apache 2.0 License](http://www.apache.org/licenses/LICENSE-2.0.html).
