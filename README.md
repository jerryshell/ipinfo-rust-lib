# <img src="https://ipinfo.io/static/ipinfo-small.svg" alt="IPinfo" width="24"/> IPinfo Rust Client Library

Fork From: [https://github.com/ipinfo/rust](https://github.com/ipinfo/rust)

Jerry Fork Edition New Features:

* Rust Edition: 2018 -> 2021
* `reqwest`: 0.9 -> 0.11, default-tls -> rustls-tls
* `lru`: 0.6 -> 0.9

## Testing

Before `cargo test`, create a `.env` file in the root directory of the project and fill in the content:

```env
IPINFO_TOKEN=your_token
```
