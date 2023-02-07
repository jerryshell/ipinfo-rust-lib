//   Copyright 2019 IPinfo library developers
//
//   Licensed under the Apache License, Version 2.0 (the "License");
//   you may not use this file except in compliance with the License.
//   You may obtain a copy of the License at
//
//       http://www.apache.org/licenses/LICENSE-2.0
//
//   Unless required by applicable law or agreed to in writing, software
//   distributed under the License is distributed on an "AS IS" BASIS,
//   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//   See the License for the specific language governing permissions and
//   limitations under the License.

use std::{collections::HashMap, fs, num::NonZeroUsize, time::Duration};

use crate::{Continent, CountryCurrency, CountryFlag, IpDetails, IpError, VERSION};

use lru::LruCache;
use serde_json::json;

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, USER_AGENT};

use include_dir::{include_dir, Dir};
static ASSETS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");

/// IpInfo structure configuration.
pub struct IpInfoConfig {
    /// IPinfo access token.
    pub token: Option<String>,

    /// The timeout of HTTP requests. (default: 3 seconds)
    pub timeout: Duration,

    /// The size of the LRU cache. (default: 100 IPs)
    pub cache_size: usize,

    /// The file path of `countries.json`
    pub countries_file_path: Option<String>,

    /// The file path of `eu.json`
    pub eu_file_path: Option<String>,

    /// The file path of `flags.json`
    pub country_flags_file_path: Option<String>,

    /// The file path of `currencies.json`
    pub country_currencies_file_path: Option<String>,

    /// The file path of `continents.json`
    pub continents_file_path: Option<String>,
}

impl Default for IpInfoConfig {
    fn default() -> Self {
        Self {
            token: None,
            timeout: Duration::from_secs(3),
            cache_size: 100,
            countries_file_path: None,
            eu_file_path: None,
            country_flags_file_path: None,
            country_currencies_file_path: None,
            continents_file_path: None,
        }
    }
}

/// IPinfo requests context structure.
pub struct IpInfo {
    url: String,
    token: Option<String>,
    client: reqwest::blocking::Client,
    cache: LruCache<String, IpDetails>,
    countries: HashMap<String, String>,
    eu: Vec<String>,
    country_flags: HashMap<String, CountryFlag>,
    country_currencies: HashMap<String, CountryCurrency>,
    continents: HashMap<String, Continent>,
}

impl IpInfo {
    /// Construct a new IpInfo structure.
    ///
    /// # Examples
    ///
    /// ```
    /// use ipinfo::IpInfo;
    ///
    /// let ipinfo = IpInfo::new(Default::default()).expect("should construct");
    /// ```
    pub fn new(config: IpInfoConfig) -> Result<Self, IpError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(config.timeout)
            .build()?;

        let url = "https://ipinfo.io".to_owned();

        let mut ipinfo_obj = Self {
            url,
            client,
            token: config.token,
            cache: LruCache::new(NonZeroUsize::new(config.cache_size).unwrap()),
            countries: HashMap::new(),
            eu: Vec::new(),
            country_flags: HashMap::new(),
            country_currencies: HashMap::new(),
            continents: HashMap::new(),
        };

        if config.countries_file_path.is_none() {
            let t_file = ASSETS_DIR
                .get_file("countries.json")
                .expect("error opening file");
            ipinfo_obj.countries =
                serde_json::from_str(t_file.contents_utf8().unwrap()).expect("error parsing JSON!");
        } else {
            let t_file = fs::File::open(config.countries_file_path.as_ref().unwrap())
                .expect("error opening file");
            ipinfo_obj.countries = serde_json::from_reader(t_file).expect("error parsing JSON!");
        }

        if config.eu_file_path.is_none() {
            let t_file = ASSETS_DIR.get_file("eu.json").expect("error opening file");
            ipinfo_obj.eu =
                serde_json::from_str(t_file.contents_utf8().unwrap()).expect("error parsing JSON!");
        } else {
            let t_file =
                fs::File::open(config.eu_file_path.as_ref().unwrap()).expect("error opening file");
            ipinfo_obj.eu = serde_json::from_reader(t_file).expect("error parsing JSON!");
        }

        if config.country_flags_file_path.is_none() {
            let t_file = ASSETS_DIR
                .get_file("flags.json")
                .expect("error opening file");
            ipinfo_obj.country_flags =
                serde_json::from_str(t_file.contents_utf8().unwrap()).expect("error parsing JSON!");
        } else {
            let t_file = fs::File::open(config.country_flags_file_path.as_ref().unwrap())
                .expect("error opening file");
            ipinfo_obj.country_flags =
                serde_json::from_reader(t_file).expect("error parsing JSON!");
        }

        if config.country_currencies_file_path.is_none() {
            let t_file = ASSETS_DIR
                .get_file("currency.json")
                .expect("error opening file");
            ipinfo_obj.country_currencies =
                serde_json::from_str(t_file.contents_utf8().unwrap()).expect("error parsing JSON!");
        } else {
            let t_file = fs::File::open(config.country_currencies_file_path.as_ref().unwrap())
                .expect("error opening file");
            ipinfo_obj.country_currencies =
                serde_json::from_reader(t_file).expect("error parsing JSON!");
        }

        if config.continents_file_path.is_none() {
            let t_file = ASSETS_DIR
                .get_file("continent.json")
                .expect("error opening file");
            ipinfo_obj.continents =
                serde_json::from_str(t_file.contents_utf8().unwrap()).expect("error parsing JSON!");
        } else {
            let t_file = fs::File::open(config.continents_file_path.as_ref().unwrap())
                .expect("error opening file");
            ipinfo_obj.continents = serde_json::from_reader(t_file).expect("error parsing JSON!");
        }

        Ok(ipinfo_obj)
    }

    /// Lookup a list of one or more IP addresses.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ipinfo::IpInfo;
    ///
    /// let mut ipinfo = IpInfo::new(Default::default()).expect("should construct");
    /// let res = ipinfo.lookup(&["8.8.8.8"]).expect("should run");
    /// ```
    pub fn lookup(&mut self, ips: &[&str]) -> Result<HashMap<String, IpDetails>, IpError> {
        let mut hits: Vec<IpDetails> = vec![];
        let mut misses: Vec<&str> = vec![];

        // Check for cache hits
        ips.iter()
            .for_each(|x| match self.cache.get(&x.to_string()) {
                Some(detail) => hits.push(detail.clone()),
                None => misses.push(*x),
            });

        // Lookup cache misses
        let response = self
            .client
            .post(&format!("{}/batch", self.url))
            .headers(Self::construct_headers())
            .bearer_auth(self.token.as_ref().unwrap_or(&"".to_string()))
            .json(&json!(misses))
            .send()?;

        // Check if we exhausted our request quota
        if let reqwest::StatusCode::TOO_MANY_REQUESTS = response.status() {
            return Err(err!(RateLimitExceededError));
        }

        // Acquire response
        let raw_resp = response.error_for_status()?.text()?;

        // Parse the response
        let resp: serde_json::Value = serde_json::from_str(&raw_resp)?;

        // Return if an error occurred
        if let Some(e) = resp["error"].as_str() {
            return Err(err!(IpRequestError, e));
        }

        // Parse the results
        let mut details: HashMap<String, IpDetails> = serde_json::from_str(&raw_resp)?;

        // Add country_name and EU status to response
        for detail in details.clone() {
            let mut_details = details.get_mut(&detail.0).unwrap();
            let country = &mut_details.country;
            if !country.is_empty() {
                let country_name = self.countries.get(&mut_details.country).unwrap();
                mut_details.country_name = Some(country_name.to_string());
                mut_details.is_eu = Some(self.eu.contains(country));
                let country_flag = self.country_flags.get(&mut_details.country).unwrap();
                mut_details.country_flag = Some(country_flag.to_owned());
                let country_currency = self.country_currencies.get(&mut_details.country).unwrap();
                mut_details.country_currency = Some(country_currency.to_owned());
                let continent = self.continents.get(&mut_details.country).unwrap();
                mut_details.continent = Some(continent.to_owned());
            }
        }

        // Update cache
        details.iter().for_each(|x| {
            self.cache.put(x.0.clone(), x.1.clone());
        });

        // Add cache hits to the result
        hits.iter().for_each(|x| {
            details.insert(x.ip.clone(), x.clone());
        });

        Ok(details)
    }

    /// Construct API request headers.
    fn construct_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&format!("IPinfoClient/Rust/{VERSION}")).unwrap(),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IpErrorKind;

    // fn get_ipinfo_client() -> IpInfo {
    //     IpInfo::new(IpInfoConfig {
    //         token: Some(std::env::var("IPINFO_TOKEN").unwrap()),
    //         timeout: Duration::from_secs(3),
    //         cache_size: 100,
    //         ..Default::default()
    //     })
    //     .expect("should construct")
    // }

    #[test]
    fn ipinfo_config_defaults_reasonable() {
        let ipinfo_config = IpInfoConfig::default();

        assert_eq!(ipinfo_config.timeout, Duration::from_secs(3));
        assert_eq!(ipinfo_config.cache_size, 100);
    }

    #[test]
    fn request_headers_are_canonical() {
        let headers = IpInfo::construct_headers();

        assert_eq!(headers[USER_AGENT], format!("IPinfoClient/Rust/{VERSION}"));
        assert_eq!(headers[CONTENT_TYPE], "application/json");
        assert_eq!(headers[ACCEPT], "application/json");
    }

    // #[test]
    // fn request_single_ip() {
    //     let mut ipinfo = get_ipinfo_client();

    //     let details = ipinfo.lookup(&["66.87.125.72"]).expect("should lookup");

    //     assert!(details.contains_key("66.87.125.72"));
    //     assert_eq!(details.len(), 1);
    // }

    #[test]
    fn request_single_ip_no_token() {
        let mut ipinfo = IpInfo::new(Default::default()).expect("should construct");

        assert_eq!(
            ipinfo.lookup(&["8.8.8.8"]).err().unwrap().kind(),
            IpErrorKind::IpRequestError
        );
    }

    // #[test]
    // fn request_multiple_ip() {
    //     let mut ipinfo = get_ipinfo_client();

    //     let details = ipinfo
    //         .lookup(&["8.8.8.8", "4.2.2.4"])
    //         .expect("should lookup");

    //     // Assert successful lookup
    //     assert!(details.contains_key("8.8.8.8"));
    //     assert!(details.contains_key("4.2.2.4"));

    //     // Assert 8.8.8.8
    //     let ip8 = &details["8.8.8.8"];
    //     assert_eq!(ip8.ip, "8.8.8.8");
    //     assert_eq!(ip8.hostname, Some("dns.google".to_owned()));
    //     assert_eq!(ip8.city, "Mountain View");
    //     assert_eq!(ip8.region, "California");
    //     assert_eq!(ip8.country, "US");
    //     assert_eq!(
    //         ip8.country_flag,
    //         Some(CountryFlag {
    //             emoji: "🇺🇸".to_owned(),
    //             unicode: "U+1F1FA U+1F1F8".to_owned()
    //         })
    //     );
    //     assert_eq!(
    //         ip8.country_currency,
    //         Some(CountryCurrency {
    //             code: "USD".to_owned(),
    //             symbol: "$".to_owned()
    //         })
    //     );
    //     assert_eq!(
    //         ip8.continent,
    //         Some(Continent {
    //             code: "NA".to_owned(),
    //             name: "North America".to_owned()
    //         })
    //     );
    //     assert_eq!(ip8.loc, "37.4056,-122.0775");
    //     assert_eq!(ip8.postal, Some("94043".to_owned()));
    //     assert_eq!(ip8.timezone, Some("America/Los_Angeles".to_owned()));

    //     // Assert 4.2.2.4
    //     let ip4 = &details["4.2.2.4"];
    //     assert_eq!(ip4.ip, "4.2.2.4");
    //     assert_eq!(ip4.hostname, Some("d.resolvers.level3.net".to_owned()));
    //     assert_eq!(ip4.city, "Monroe");
    //     assert_eq!(ip4.region, "Louisiana");
    //     assert_eq!(ip4.country, "US");
    //     assert_eq!(ip4.loc, "32.5530,-92.0422");
    //     assert_eq!(ip4.postal, Some("71203".to_owned()));
    //     assert_eq!(ip4.timezone, Some("America/Chicago".to_owned()));
    // }

    // #[test]
    // fn request_cache_miss_and_hit() {
    //     let mut ipinfo = get_ipinfo_client();

    //     // Populate the cache with 8.8.8.8
    //     let details = ipinfo.lookup(&["8.8.8.8"]).expect("should lookup");

    //     // Assert 1 result
    //     assert!(details.contains_key("8.8.8.8"));
    //     assert_eq!(details.len(), 1);

    //     // Should have a cache hit for 8.8.8.8 and query for 4.2.2.4
    //     let details = ipinfo
    //         .lookup(&["4.2.2.4", "8.8.8.8"])
    //         .expect("should lookup");

    //     // Assert 2 results
    //     assert!(details.contains_key("8.8.8.8"));
    //     assert!(details.contains_key("4.2.2.4"));
    //     assert_eq!(details.len(), 2);
    // }
}
