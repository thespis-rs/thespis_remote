# thespis_remote

[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)
[![Build Status](https://github.com/najamelan/thespis_remote/workflows/ci/badge.svg?branch=release)](https://github.com/najamelan/thespis_remote/actions)
[![Docs](https://docs.rs/thespis_remote/badge.svg)](https://docs.rs/thespis_remote)
[![crates.io](https://img.shields.io/crates/v/thespis_remote.svg)](https://crates.io/crates/thespis_remote)


> Remote functionality for thespis actors.

_thespis_remote_ allows you to send actor messages to remote processes. The requirement is a connection that can implement AsyncRead/AsyncWrite. The remote process exposes `Service`s, rather than individual actors. On each connection one can specify exactly which services are exposed.

## Table of Contents

- [Install](#install)
   - [Upgrade](#upgrade)
   - [Dependencies](#dependencies)
   - [Security](#security)
- [Usage](#usage)
   - [Basic Example](#basic-example)
   - [API](#api)
- [Contributing](#contributing)
   - [Code of Conduct](#code-of-conduct)
- [License](#license)


## Install
With [cargo add](https://github.com/killercup/cargo-edit):
`cargo add thespis_remote`

With [cargo yaml](https://gitlab.com/storedbox/cargo-yaml):
```yaml
dependencies:

   thespis_remote: ^0.1
```

In Cargo.toml:
```toml
[dependencies]

   thespis_remote = "0.1"
```

### Upgrade

Please check out the [changelog](https://github.com/najamelan/thespis_remote/blob/release/CHANGELOG.md) when upgrading.


### Dependencies

This crate has few dependencies. Cargo will automatically handle it's dependencies for you.

There are no optional features.


### Security

It is recommended to always use [cargo-crev](https://github.com/crev-dev/cargo-crev) to verify the trustworthiness of each of your dependencies, including this one.


## Usage



### Basic example

```rust

```

## API

API documentation can be found on [docs.rs](https://docs.rs/thespis_remote).


## Contributing

Please check out the [contribution guidelines](https://github.com/najamelan/thespis_remote/blob/release/CONTRIBUTING.md).


### Testing


### Code of conduct

Any of the behaviors described in [point 4 "Unacceptable Behavior" of the Citizens Code of Conduct](https://github.com/stumpsyn/policies/blob/master/citizen_code_of_conduct.md#4-unacceptable-behavior) are not welcome here and might get you banned. If anyone, including maintainers and moderators of the project, fail to respect these/your limits, you are entitled to call them out.

## License

[Unlicence](https://unlicense.org/)

