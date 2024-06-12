# ConfigCat SDK for Rust

[![Build Status](https://github.com/configcat/rust-sdk/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/configcat/rust-sdk/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/configcat.svg?logo=rust)](https://crates.io/crates/configcat)
[![docs.rs](https://img.shields.io/badge/docs.rs-configcat-66c2a5?logo=docs.rs)](https://docs.rs/configcat)

ConfigCat SDK for Rust provides easy integration for your application to [ConfigCat](https://configcat.com).

## Getting started

### 1. Install the package

Run the following Cargo command in your project directory:
```shell
cargo add configcat
```

Or add the following to your `Cargo.toml`:

```toml
[dependencies]
configcat = "0.1"
```

### 2. Go to the <a href="https://app.configcat.com/sdkkey" target="_blank">ConfigCat Dashboard</a> to get your *SDK Key*:
![SDK-KEY](https://raw.githubusercontent.com/configcat/rust-sdk/main/media/readme02-3.png  "SDK-KEY")

### 3. Import the `configcat` module to your application
```rust
use configcat::*;
```

### 4. Create a *ConfigCat* client instance
```rust
use configcat::*;

#[tokio::main]
async fn main() {
    let client = Client::new("#YOUR-SDK-KEY#").unwrap();
}
```

### 5. Get your setting value
```rust
use configcat::*;

#[tokio::main]
async fn main() {
    let client = Client::new("#YOUR-SDK-KEY#").unwrap();

    let is_awesome_feature_enabled = client.get_value("isAwesomeFeatureEnabled", false, None).await;
    
    if is_awesome_feature_enabled {
        do_the_new_thing();
    } else {
        do_the_old_thing();
    }
}
```

## Getting user specific setting values with Targeting
Using this feature, you will be able to get different setting values for different users in your application by passing a `User Object` to the `get_value()` function.

Read more about [Targeting here](https://configcat.com/docs/advanced/targeting/).

```rust
use configcat::*;

#[tokio::main]
async fn main() {
    let client = Client::new("#YOUR-SDK-KEY#").unwrap();

    let user = User::new("#USER-IDENTIFIER#");
    let is_awesome_feature_enabled = client.get_value("isAwesomeFeatureEnabled", false, Some(user)).await;

    if is_awesome_feature_enabled {
        do_the_new_thing();
    } else {
        do_the_old_thing();
    }
}
```

## Example

This repository contains a simple [example application](./examples/print_eval.rs) that you can run with:
```shell
cargo run --example print_eval
```

## Polling Modes
The ConfigCat SDK supports 3 different polling mechanisms to acquire the setting values from ConfigCat. After latest setting values are downloaded, they are stored in the internal cache then all requests are served from there. Read more about Polling Modes and how to use them at [ConfigCat Docs](https://configcat.com/docs/sdk-reference/rust).

## Need help?
https://configcat.com/support

## Contributing
Contributions are welcome. For more info please read the [Contribution Guideline](CONTRIBUTING.md).

## About ConfigCat
ConfigCat is a feature flag and configuration management service that lets you separate releases from deployments. You can turn your features ON/OFF using <a href="https://app.configcat.com" target="_blank">ConfigCat Dashboard</a> even after they are deployed. ConfigCat lets you target specific groups of users based on region, email or any other custom user attribute.

ConfigCat is a <a href="https://configcat.com" target="_blank">hosted feature flag service</a>. Manage feature toggles across frontend, backend, mobile, desktop apps. <a href="https://configcat.com" target="_blank">Alternative to LaunchDarkly</a>. Management app + feature flag SDKs.

- [Official ConfigCat SDKs for other platforms](https://github.com/configcat)
- [Documentation](https://configcat.com/docs)
- [Blog](https://configcat.com/blog)
