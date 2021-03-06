# dness

***Finesse with dness: a dynamic dns client***

[![Build Status](https://travis-ci.org/nickbabcock/dness.svg?branch=master)](https://travis-ci.org/nickbabcock/dness) [![Build status](https://ci.appveyor.com/api/projects/status/dic59x43w2g19536?svg=true)](https://ci.appveyor.com/project/nickbabcock/dness)

---

## Motivation

When one has a server that is subjected to unpredictable IP address changes, such as at home or elsewhere, a change in IP address causes unexpected downtime. Instead of paying for a static IP address, one can employ a dynamic dns client on said server, which will update the [WAN](https://en.wikipedia.org/wiki/Wide_area_network) IP address on the dns server.

There are plenty of dynamic dns clients, including the venerable [ddclient](https://github.com/ddclient/ddclient), but troublesome installation + perl system dependency resolution, and cache format errors have left much to be desired. Other solutions fall short, so dness was created with the following goals:

**Goals**:

- Cross platform (Windows, Mac, Linux, ARM, BSD)
- "zero" dependencies
  - Depend on only already installed system wide dependencies (libssl (eg: openssl))
  - And offer statically linked builds for truly zero dependencies
- A standard configuration ([TOML](https://github.com/toml-lang/toml)) that is similar to ddclient's.
- Extendable to allow for more dynamic dns services
- Sensible logging to glean insight into inevitable problems that arise with networked services
- Permissively licensed

## Installation

To maximize initial flexibility, dness is not a daemon. Instead it relies on the host's scheduling (cron, systemd timers, windows scheduler).

### Ubuntu / Debian (systemd + deb)

- Decide if you want a static musl package or one that depends on the system's openssl.
- Download the [latest chosen deb](https://github.com/nickbabcock/dness/releases/latest)

```bash
dpkg -i dness<version>_amd64.deb

# ensure it is working
/usr/bin/dness

# enable systemd timer
systemctl daemon-reload
systemctl start dness.timer
systemctl enable dness.timer

# update configuration
${EDITOR:-vi} /etc/dness/dness.conf
```

### Linux Musl

The linux musl build is a static build of dness. It has zero dependencies. Nothing is quite like scp'ing or curl'ing a musl binary to a random / unknown linux server and having it just work.

- Download the [latest "-x86_64-unknown-linux-musl.tar.gz" ](https://github.com/nickbabcock/dness/releases/latest)
- untar (`tar -xzf *-x86_64-unknown-linux-musl.tar.gz`)
- enjoy

### Windows

- Download the [latest ".exe"](https://github.com/nickbabcock/dness/releases/latest)
- Create configuration file (`dness.conf`)
- Execute `dness.exe -c dness.conf` to verify behavior
- If desired, use windows task scheduler to execute command at specific times

### Other

Download the [latest appropriate target](https://github.com/nickbabcock/dness/releases/latest)

## Configuration

No configuration file is necessary when only the WAN IP is desired.

```bash
./dness
```

Sample output:

```
[INFO  trust_dns_proto::xfer::dns_exchange] sending message via: UDP(208.67.220.220:53)
[INFO  dness] resolved address to 256.256.256.256 in 23ms
[INFO  dness] processed all: (updated: 0, already current: 0, missing: 0) in 29ms
```

### Sample Configuration

But dness can do more than resolve one's WAN IP. Below is a sample configuration (dness.conf) that should cover most needs:

```toml
[log]
# How verbose the log is. Common values: Error, Warn, Info, Debug, Trace
# The default level is info
level = "Info"

[[domains]]
# We denote that our domain is managed by cloudflare
type = "cloudflare"

# The email address registered in cloudflare that is authorized to update dns
# records
email = "admin@example.com"

# The cloudflare key can be found in the domain overview, in "Get your API key"
# and view "Global API Key" (or another key as appropriate)
key = "deadbeef"

# The zone is the domain name
zone = "example.com"

# List of A records found under the DNS tab that should be updated
records = [
    "n.example.com"
]

# More than one domain can be specified in a config!
[[domains]]
type = "cloudflare"
email = "admin@example.com"
key = "deadbeef"
zone = "example2.com"
records = [
    "n.example2.com",
    "n2.example2.com"
]
```

Execute with configuration:

```
./dness -c dness.conf
```

### Supported Dynamic DNS Services

#### Cloudflare

```toml
[[domains]]
# We denote that our domain is managed by cloudflare
type = "cloudflare"

# The email address registered in cloudflare that is authorized to update dns
# records
email = "admin@example.com"

# The cloudflare key can be found in the domain overview, in "Get your API key"
# and view "Global API Key" (or another key as appropriate)
key = "deadbeef"

# The zone is the domain name
zone = "example.com"

# List of A records found under the DNS tab that should be updated
records = [
    "n.example.com"
]
```

Cloudflare dynamic dns service works in three steps:

1. Send GET to translate the zone (example.com) to cloudflare's id
2. Send GET to find all the domains under the zone and their sub-ids
   - Cloudflare paginates the response to handle many subdomains
   - It is possible to query for individual domains but as long as more
     than one desired domain in each page -- this methods cuts down requests
3. Each desired domain in the config is checked to ensure that it is set to our address. In
   this way cloudflare is our cache (to guard against nefarious users updating out of band)

#### GoDaddy

```toml
[[domains]]
# denote that the domain is managed by godaddy
type = "godaddy"

# The GoDaddy domain: https://dcc.godaddy.com/domains/
domain = "example.com"

# This is the api key, you can create one here:
# https://developer.godaddy.com/keys
key = "abc123"

# The password for the key, top secret!
secret = "ef"

# The records to update. "@" = "example.com", "a" = "a.example.com"
records = [ "@", "a" ]
```

GoDaddy dynamic dns service works as the following:

1. Send a GET request to find all records in the domain
2. Find all the expected records (and log those that are missing) and check their current IP
3. Update the remote IP as needed, ensuring that original properties are preserved in the upload, so that we don't overwrite a property like TTL.

#### Namecheap

```toml
# Namecheap requires that dynamic dns is enabled in their UI!
type = "namecheap"
domain = "test-dness-1.xyz"
ddns_password = "super_secret_password"

# The records to update. Make sure they are listed as A + Dynamic DNS
# "@" = "test-dness-1.xyz"
# "* = "<any-sub-domain>.test-dness-1.xyz"
# "sub = "sub.test-dness-1.xyz"
records = [ "@", "*", "sub" ]
```

The namecheap services requires dynamic dns enabled in their UI.

Updating the dns entry works as follows:

- A dns query is sent to cloudflare to check the IP of the record
- If the IP is different than WAN then a request is sent to namecheap to update it
- If the IP is the same, no action is taken

This method suffers from natural flow of dns propagation. When namecheap receives the update, it may take up to an hour for cloudflare to see the new record. In the meantime, dness will keep updating namecheap servers with the WAN. This has no consequential side effects other than momentary confusion why updates are being sent to namecheap every 5 minutes. Future revisions of this provider may use another method (like API integration) if the current method proves deficient enough.

### Supported WAN IP Resolvers

There are a couple different methods for dness to resolve the WAN IP address.

#### OpenDNS

The default WAN IP address resolver queries OpenDNS. It resolves IPv4 addresses by querying "myip.opendns.com" against resolver1.opendns.com and resolver2.opendns.com.

While it is the default, it can explicitly be specified by appending snippet below to the top of the config:

```toml
ip_resolver = "opendns"
```

#### Ipify

OpenDNS may not be available to all networks, so one can configure dness to use [Ipify](https://www.ipify.org/). Instead of using DNS, an HTTPs request will be sent. To opt into using Ipify, append the snippet below to the top of the config:

```toml
ip_resolver = "ipify"
```
