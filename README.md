Rust HTTP Parser using Nom
==========================

[![Build Status](https://travis-ci.org/erikjohnston/rust-nom-http.svg?branch=master)](https://travis-ci.org/erikjohnston/rust-nom-http)

_**This is me messing around with Rust and Nom. Don't even think about using
this!**_

Stuff left to do:
- [x] Requests
- [x] Body types
  - [x] EOF
  - [x] Content-Length
  - [x] Chunked
    - [x] Consume and expose trailing headers
    - [x] Consume chunk params
    - [ ] Expose chunk params in API
- [x] Responses
- [ ] Fixup API
  - [x] Proper error handling
  - [x] Allow parser to be reused
- [ ] Ability to pause parser
- [ ] Ability to consume, but discard, the rest of the HTTP Message
